//! Data access for label records.
//!
//! Port of the Go `internal/compliance/labels/repository.go`. `NUMERIC` columns
//! (`abv_percent`, `energy_kj_per_100ml`, `energy_kcal_per_100ml`,
//! `alcohol_units_per_serving`) are selected as `float8`; the `DATE` column
//! (`best_before_date`) is rendered with `to_char(..., 'YYYY-MM-DD')`. The
//! `allergens` column is a Postgres `TEXT[]` (mapped to `Vec<String>`). All
//! domain queries are tenant-scoped; missing/cross-tenant rows become 404 and
//! the unique `(tenant_id, batch_id)` violation becomes 409.

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{LabelRecord, ListFilter, Page};
use crate::platform::errors::ApiError;

/// Selected columns for a label record, with `NUMERIC`→`float8` casts and the
/// `DATE` rendered as a `YYYY-MM-DD` string.
const LABEL_COLS: &str = "id, tenant_id, batch_id, status, \
    product_name, abv_percent::float8 AS abv_percent, allergens, net_volume_ml, \
    responsible_party, country_of_origin, \
    to_char(best_before_date, 'YYYY-MM-DD') AS best_before_date, lot_identifier, \
    ingredient_list, \
    energy_kj_per_100ml::float8 AS energy_kj_per_100ml, \
    energy_kcal_per_100ml::float8 AS energy_kcal_per_100ml, \
    alcohol_units_per_serving::float8 AS alcohol_units_per_serving, \
    serving_volume_ml, created_at, updated_at";

fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 { 20 } else { page_size };
    (page, page_size)
}

/// The subset of batch data the label creator reads from the recipe snapshot.
pub struct BatchInfo {
    pub recipe_id: Option<Uuid>,
    pub batch_number: String,
    pub product_name: String,
    pub abv_percent: f64,
}

/// Fetches the batch fields needed to auto-populate a label record, tenant-scoped.
/// `product_name` and `abv_percent` come from the JSONB `batch_recipe_snapshot`.
pub async fn select_batch_info(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<Option<BatchInfo>, sqlx::Error> {
    let row: Option<(Option<Uuid>, String, String, Option<f64>)> = sqlx::query_as(
        "SELECT recipe_id, batch_number, \
            batch_recipe_snapshot->>'name', \
            (batch_recipe_snapshot->>'calc_abv_pct')::float8 \
         FROM batches WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(batch_id)
    .fetch_optional(pool)
    .await?;
    Ok(
        row.map(|(recipe_id, batch_number, product_name, abv)| BatchInfo {
            recipe_id,
            batch_number,
            product_name,
            abv_percent: abv.unwrap_or(0.0),
        }),
    )
}

/// Scalar inputs for inserting a label record. `id`/timestamps come from the DB.
pub struct LabelInsert {
    pub batch_id: Uuid,
    pub status: String,
    pub product_name: String,
    pub abv_percent: f64,
    pub allergens: Vec<String>,
    pub net_volume_ml: i32,
    pub responsible_party: String,
    pub country_of_origin: String,
    pub best_before_date: Option<String>,
    pub lot_identifier: String,
    pub ingredient_list: Option<String>,
    pub energy_kj_per_100ml: Option<f64>,
    pub energy_kcal_per_100ml: Option<f64>,
    pub alcohol_units_per_serving: Option<f64>,
    pub serving_volume_ml: Option<i32>,
}

/// Inserts a label record and returns the persisted row. A duplicate
/// `(tenant_id, batch_id)` becomes a 409 conflict.
pub async fn insert(
    pool: &PgPool,
    tenant_id: Uuid,
    rec: &LabelInsert,
) -> Result<LabelRecord, ApiError> {
    let sql = format!(
        "INSERT INTO label_records ( \
            tenant_id, batch_id, status, \
            product_name, abv_percent, allergens, net_volume_ml, \
            responsible_party, country_of_origin, best_before_date, lot_identifier, \
            ingredient_list, energy_kj_per_100ml, energy_kcal_per_100ml, \
            alcohol_units_per_serving, serving_volume_ml \
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::date,$11,$12,$13,$14,$15,$16) \
         RETURNING {LABEL_COLS}"
    );
    sqlx::query_as::<_, LabelRecord>(&sql)
        .bind(tenant_id)
        .bind(rec.batch_id)
        .bind(&rec.status)
        .bind(&rec.product_name)
        .bind(rec.abv_percent)
        .bind(&rec.allergens)
        .bind(rec.net_volume_ml)
        .bind(&rec.responsible_party)
        .bind(&rec.country_of_origin)
        .bind(&rec.best_before_date)
        .bind(&rec.lot_identifier)
        .bind(&rec.ingredient_list)
        .bind(rec.energy_kj_per_100ml)
        .bind(rec.energy_kcal_per_100ml)
        .bind(rec.alcohol_units_per_serving)
        .bind(rec.serving_volume_ml)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db) = &e {
                if db.is_unique_violation() {
                    return ApiError::conflict(
                        "label_record",
                        "a label record already exists for this batch",
                    );
                }
            }
            ApiError::internal(e)
        })
}

/// Fetches a label record by id, tenant-scoped.
pub async fn select_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<LabelRecord>, sqlx::Error> {
    let sql = format!("SELECT {LABEL_COLS} FROM label_records WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, LabelRecord>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Fetches the most recently updated approved label record for a batch.
pub async fn select_latest_approved_for_batch(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<Option<LabelRecord>, sqlx::Error> {
    let sql = format!(
        "SELECT {LABEL_COLS} FROM label_records \
         WHERE tenant_id=$1 AND batch_id=$2 AND status='approved' \
         ORDER BY updated_at DESC LIMIT 1"
    );
    sqlx::query_as::<_, LabelRecord>(&sql)
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_optional(pool)
        .await
}

/// Lists label records with filters, sorting, and pagination.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
) -> Result<Page<LabelRecord>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(b) = filter.batch_id {
            qb.push(" AND batch_id = ").push_bind(b);
        }
        if let Some(s) = &filter.status {
            if !s.is_empty() {
                qb.push(" AND status = ").push_bind(s.clone());
            }
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM label_records");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    // Mirror the Go whitelist: a `-` prefix means DESC, default `created_at DESC`.
    let order_by = match filter.sort.as_deref() {
        Some("created_at") => "created_at ASC",
        Some("-created_at") => "created_at DESC",
        Some("updated_at") => "updated_at ASC",
        Some("-updated_at") => "updated_at DESC",
        Some("product_name") => "product_name ASC",
        Some("-product_name") => "product_name DESC",
        Some("status") => "status ASC",
        Some("-status") => "status DESC",
        _ => "created_at DESC",
    };

    let mut list_qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {LABEL_COLS} FROM label_records"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb
        .build_query_as::<LabelRecord>()
        .fetch_all(pool)
        .await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Replaces the full mutable column set of a label record, tenant-scoped.
pub async fn update_full(
    pool: &PgPool,
    tenant_id: Uuid,
    rec: &LabelRecord,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE label_records SET \
            status=$1, \
            product_name=$2, abv_percent=$3, allergens=$4, net_volume_ml=$5, \
            responsible_party=$6, country_of_origin=$7, best_before_date=$8::date, lot_identifier=$9, \
            ingredient_list=$10, energy_kj_per_100ml=$11, energy_kcal_per_100ml=$12, \
            alcohol_units_per_serving=$13, serving_volume_ml=$14, \
            updated_at=now() \
         WHERE tenant_id=$15 AND id=$16",
    )
    .bind(&rec.status)
    .bind(&rec.product_name)
    .bind(rec.abv_percent)
    .bind(&rec.allergens)
    .bind(rec.net_volume_ml)
    .bind(&rec.responsible_party)
    .bind(&rec.country_of_origin)
    .bind(&rec.best_before_date)
    .bind(&rec.lot_identifier)
    .bind(&rec.ingredient_list)
    .bind(rec.energy_kj_per_100ml)
    .bind(rec.energy_kcal_per_100ml)
    .bind(rec.alcohol_units_per_serving)
    .bind(rec.serving_volume_ml)
    .bind(tenant_id)
    .bind(rec.id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a label record by id, tenant-scoped. Returns whether a row was removed.
pub async fn delete_by_id(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM label_records WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}
