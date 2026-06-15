//! Data access for batches and batch ingredients.
//!
//! Port of the Go `internal/batch/repository.go`. The status FSM is enforced by
//! a DB trigger on UPDATE; the service also checks it in code (defence in depth).

use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::{PgConnection, PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Batch, BatchIngredient, BatchRecipeSnapshot, ListFilter, Page};

const BAT_COLS: &str = "id, tenant_id, recipe_id, batch_number, name, status, \
    to_char(brew_date, 'YYYY-MM-DD') AS brew_date, to_char(package_date, 'YYYY-MM-DD') AS package_date, \
    target_og::float8 AS target_og, actual_og::float8 AS actual_og, target_fg::float8 AS target_fg, \
    actual_fg::float8 AS actual_fg, actual_volume_liters::float8 AS actual_volume_liters, notes, \
    duty_status, batch_recipe_snapshot, created_at, updated_at";

/// Values for inserting a new batch.
pub struct NewBatch {
    pub recipe_id: Option<Uuid>,
    pub batch_number: String,
    pub name: String,
    pub status: String,
    pub brew_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub duty_status: String,
    pub target_og: Option<f64>,
    pub target_fg: Option<f64>,
    pub snapshot: BatchRecipeSnapshot,
}

/// Mutable fields updated by PATCH/PUT.
pub struct BatchMutable {
    pub name: String,
    pub brew_date: Option<NaiveDate>,
    pub package_date: Option<NaiveDate>,
    pub target_og: Option<f64>,
    pub actual_og: Option<f64>,
    pub target_fg: Option<f64>,
    pub actual_fg: Option<f64>,
    pub actual_volume_liters: Option<f64>,
    pub notes: Option<String>,
}

fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 {
        20
    } else if page_size > 100 {
        100
    } else {
        page_size
    };
    (page, page_size)
}

/// Inserts a batch and returns the created row.
pub async fn insert<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    b: &NewBatch,
) -> Result<Batch, sqlx::Error> {
    let sql = format!(
        "INSERT INTO batches (tenant_id, recipe_id, batch_number, name, status, brew_date, notes, \
         duty_status, target_og, target_fg, batch_recipe_snapshot) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11::jsonb) RETURNING {BAT_COLS}"
    );
    sqlx::query_as::<_, Batch>(&sql)
        .bind(tenant_id)
        .bind(b.recipe_id)
        .bind(&b.batch_number)
        .bind(&b.name)
        .bind(&b.status)
        .bind(b.brew_date)
        .bind(&b.notes)
        .bind(&b.duty_status)
        .bind(b.target_og)
        .bind(b.target_fg)
        .bind(sqlx::types::Json(&b.snapshot))
        .fetch_one(exec)
        .await
}

/// Fetches a batch by id, tenant-scoped.
pub async fn select_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Batch>, sqlx::Error> {
    let sql = format!("SELECT {BAT_COLS} FROM batches WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Batch>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Fetches a batch by batch_number, tenant-scoped.
pub async fn select_by_batch_number(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_number: &str,
) -> Result<Option<Batch>, sqlx::Error> {
    let sql = format!("SELECT {BAT_COLS} FROM batches WHERE tenant_id=$1 AND batch_number=$2");
    sqlx::query_as::<_, Batch>(&sql)
        .bind(tenant_id)
        .bind(batch_number)
        .fetch_optional(pool)
        .await
}

/// Fetches a batch `FOR UPDATE` within a transaction.
pub async fn select_for_update<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Batch>, sqlx::Error> {
    let sql = format!("SELECT {BAT_COLS} FROM batches WHERE tenant_id=$1 AND id=$2 FOR UPDATE");
    sqlx::query_as::<_, Batch>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(exec)
        .await
}

/// Lists batches with filters and pagination.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
    order_by: &str,
) -> Result<Page<Batch>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(s) = &filter.status {
            qb.push(" AND status = ").push_bind(s.clone());
        }
        if let Some(r) = filter.recipe_id {
            qb.push(" AND recipe_id = ").push_bind(r);
        }
        if let Some(f) = &filter.brew_date_from {
            qb.push(" AND brew_date >= ")
                .push_bind(f.clone())
                .push("::date");
        }
        if let Some(t) = &filter.brew_date_to {
            qb.push(" AND brew_date <= ")
                .push_bind(t.clone())
                .push("::date");
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM batches");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb = QueryBuilder::<Postgres>::new(format!("SELECT {BAT_COLS} FROM batches"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb.build_query_as::<Batch>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Updates a batch's mutable fields.
pub async fn update_mutable<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    m: &BatchMutable,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE batches SET name=$3, brew_date=$4, package_date=$5, target_og=$6, actual_og=$7, \
         target_fg=$8, actual_fg=$9, actual_volume_liters=$10, notes=$11, updated_at=now() \
         WHERE tenant_id=$1 AND id=$2",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&m.name)
    .bind(m.brew_date)
    .bind(m.package_date)
    .bind(m.target_og)
    .bind(m.actual_og)
    .bind(m.target_fg)
    .bind(m.actual_fg)
    .bind(m.actual_volume_liters)
    .bind(&m.notes)
    .execute(exec)
    .await
    .map(|_| ())
}

/// Updates a batch's status (fires the FSM trigger).
pub async fn update_status<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE batches SET status=$3, updated_at=now() WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .bind(status)
        .execute(exec)
        .await
        .map(|_| ())
}

/// Deletes a batch; returns true if a row was removed.
pub async fn delete_by_id<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM batches WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Replaces the batch recipe snapshot JSON.
pub async fn update_snapshot<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    snapshot: &BatchRecipeSnapshot,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE batches SET batch_recipe_snapshot=$3::jsonb, updated_at=now() WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .bind(sqlx::types::Json(snapshot))
        .execute(exec)
        .await
        .map(|_| ())
}

/// Records an ingredient deduction against a batch.
pub async fn insert_batch_ingredient(
    conn: &mut PgConnection,
    bi: &BatchIngredient,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO batch_ingredients (batch_id, ingredient_id, amount_deducted, unit, cost_pence) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(bi.batch_id)
    .bind(bi.ingredient_id)
    .bind(bi.amount_deducted)
    .bind(&bi.unit)
    .bind(bi.cost_pence)
    .execute(&mut *conn)
    .await
    .map(|_| ())
}

/// Increments the tenant's next batch number counter (no-op if NULL).
pub async fn increment_tenant_batch_number<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tenants SET next_batch_number = next_batch_number + 1 \
         WHERE id=$1 AND next_batch_number IS NOT NULL",
    )
    .bind(tenant_id)
    .execute(exec)
    .await
    .map(|_| ())
}

/// Counts batches per status; all eight statuses are present (0-filled).
pub async fn count_by_status(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<HashMap<String, i64>, sqlx::Error> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT status, COUNT(*) FROM batches WHERE tenant_id=$1 GROUP BY status")
            .bind(tenant_id)
            .fetch_all(pool)
            .await?;
    let mut out: HashMap<String, i64> = [
        "planned",
        "brewing",
        "fermenting",
        "conditioning",
        "packaging",
        "completed",
        "cancelled",
        "spoiled",
    ]
    .iter()
    .map(|s| (s.to_string(), 0))
    .collect();
    for (status, count) in rows {
        out.insert(status, count);
    }
    Ok(out)
}
