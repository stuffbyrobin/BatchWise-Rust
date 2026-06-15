//! Data access for yeast bank entries and propagation events.
//!
//! Port of the Go `internal/yeastbanking/repository.go`. `viability_percent`,
//! `quantity_ml`, `storage_temp_c`, `volume_ml` are `NUMERIC` (selected as
//! `float8`). Every query is tenant-scoped.

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Page, Propagation, YeastBankEntry, YeastBankFilter};

const ENTRY_COLS: &str = "id, tenant_id, name, library_yeast_id, generation, harvested_at, \
    viability_percent::float8 AS viability_percent, quantity_ml::float8 AS quantity_ml, \
    storage_temp_c::float8 AS storage_temp_c, location, status, notes, created_at, updated_at";

const PROP_COLS: &str = "id, tenant_id, yeast_bank_id, batch_id, started_at, completed_at, \
    volume_ml::float8 AS volume_ml, notes, created_at";

/// Matches the Go `normPage`: page<1→1; page_size<1 OR page_size>100 → 20.
fn norm_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if !(1..=100).contains(&page_size) {
        20
    } else {
        page_size
    };
    (page, page_size)
}

fn entry_order_by(sort: &str) -> &'static str {
    match sort {
        "name" => "name ASC",
        "-name" => "name DESC",
        "created_at" => "created_at ASC",
        _ => "created_at DESC",
    }
}

// ---- yeast bank entries ----

/// Inserts a yeast bank entry and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_entry(
    pool: &PgPool,
    tenant_id: Uuid,
    name: &str,
    library_yeast_id: Option<Uuid>,
    generation: i32,
    harvested_at: Option<chrono::DateTime<chrono::Utc>>,
    viability_percent: Option<f64>,
    quantity_ml: Option<f64>,
    storage_temp_c: Option<f64>,
    location: Option<&str>,
    status: &str,
    notes: Option<&str>,
) -> Result<YeastBankEntry, sqlx::Error> {
    let sql = format!(
        "INSERT INTO yeast_bank \
         (tenant_id, name, library_yeast_id, generation, harvested_at, viability_percent, \
          quantity_ml, storage_temp_c, location, status, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING {ENTRY_COLS}"
    );
    sqlx::query_as::<_, YeastBankEntry>(&sql)
        .bind(tenant_id)
        .bind(name)
        .bind(library_yeast_id)
        .bind(generation)
        .bind(harvested_at)
        .bind(viability_percent)
        .bind(quantity_ml)
        .bind(storage_temp_c)
        .bind(location)
        .bind(status)
        .bind(notes)
        .fetch_one(pool)
        .await
}

/// Lists yeast bank entries with optional status/library_yeast_id filters.
pub async fn select_entries(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &YeastBankFilter,
) -> Result<Page<YeastBankEntry>, sqlx::Error> {
    let (page, page_size) = norm_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(st) = &filter.status {
            qb.push(" AND status = ").push_bind(st.clone());
        }
        if let Some(lid) = filter.library_yeast_id {
            qb.push(" AND library_yeast_id = ").push_bind(lid);
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM yeast_bank");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = entry_order_by(&filter.sort);
    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {ENTRY_COLS} FROM yeast_bank"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by}"));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb
        .build_query_as::<YeastBankEntry>()
        .fetch_all(pool)
        .await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a yeast bank entry by id, tenant-scoped.
pub async fn select_entry_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<YeastBankEntry>, sqlx::Error> {
    let sql = format!("SELECT {ENTRY_COLS} FROM yeast_bank WHERE tenant_id = $1 AND id = $2");
    sqlx::query_as::<_, YeastBankEntry>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Updates a yeast bank entry's mutable fields and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn update_entry(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    name: &str,
    library_yeast_id: Option<Uuid>,
    generation: i32,
    harvested_at: Option<chrono::DateTime<chrono::Utc>>,
    viability_percent: Option<f64>,
    quantity_ml: Option<f64>,
    storage_temp_c: Option<f64>,
    location: Option<&str>,
    status: &str,
    notes: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE yeast_bank SET name = $3, library_yeast_id = $4, generation = $5, \
         harvested_at = $6, viability_percent = $7, quantity_ml = $8, storage_temp_c = $9, \
         location = $10, status = $11, notes = $12, updated_at = now() \
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(name)
    .bind(library_yeast_id)
    .bind(generation)
    .bind(harvested_at)
    .bind(viability_percent)
    .bind(quantity_ml)
    .bind(storage_temp_c)
    .bind(location)
    .bind(status)
    .bind(notes)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a yeast bank entry; returns true if a row was removed.
pub async fn delete_entry(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM yeast_bank WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---- propagations ----

/// Inserts a propagation event and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_propagation(
    pool: &PgPool,
    tenant_id: Uuid,
    yeast_bank_id: Uuid,
    batch_id: Option<Uuid>,
    started_at: chrono::DateTime<chrono::Utc>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    volume_ml: Option<f64>,
    notes: Option<&str>,
) -> Result<Propagation, sqlx::Error> {
    let sql = format!(
        "INSERT INTO yeast_propagations \
         (tenant_id, yeast_bank_id, batch_id, started_at, completed_at, volume_ml, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING {PROP_COLS}"
    );
    sqlx::query_as::<_, Propagation>(&sql)
        .bind(tenant_id)
        .bind(yeast_bank_id)
        .bind(batch_id)
        .bind(started_at)
        .bind(completed_at)
        .bind(volume_ml)
        .bind(notes)
        .fetch_one(pool)
        .await
}

/// Lists propagations for a yeast bank entry (newest first), tenant+bank-scoped.
pub async fn select_propagations(
    pool: &PgPool,
    tenant_id: Uuid,
    bank_id: Uuid,
    page: i64,
    page_size: i64,
) -> Result<Page<Propagation>, sqlx::Error> {
    let (page, page_size) = norm_page(page, page_size);
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM yeast_propagations WHERE tenant_id = $1 AND yeast_bank_id = $2",
    )
    .bind(tenant_id)
    .bind(bank_id)
    .fetch_one(pool)
    .await?;

    let sql = format!(
        "SELECT {PROP_COLS} FROM yeast_propagations \
         WHERE tenant_id = $1 AND yeast_bank_id = $2 \
         ORDER BY started_at DESC LIMIT $3 OFFSET $4"
    );
    let items = sqlx::query_as::<_, Propagation>(&sql)
        .bind(tenant_id)
        .bind(bank_id)
        .bind(page_size)
        .bind((page - 1) * page_size)
        .fetch_all(pool)
        .await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a propagation by (tenant, bank, prop) id.
pub async fn select_propagation_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    bank_id: Uuid,
    prop_id: Uuid,
) -> Result<Option<Propagation>, sqlx::Error> {
    let sql = format!(
        "SELECT {PROP_COLS} FROM yeast_propagations \
         WHERE tenant_id = $1 AND yeast_bank_id = $2 AND id = $3"
    );
    sqlx::query_as::<_, Propagation>(&sql)
        .bind(tenant_id)
        .bind(bank_id)
        .bind(prop_id)
        .fetch_optional(pool)
        .await
}

/// Updates a propagation's mutable fields.
#[allow(clippy::too_many_arguments)]
pub async fn update_propagation(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    batch_id: Option<Uuid>,
    started_at: chrono::DateTime<chrono::Utc>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    volume_ml: Option<f64>,
    notes: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE yeast_propagations SET batch_id = $3, started_at = $4, completed_at = $5, \
         volume_ml = $6, notes = $7 WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(batch_id)
    .bind(started_at)
    .bind(completed_at)
    .bind(volume_ml)
    .bind(notes)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a propagation by (tenant, prop) id (matches the Go signature).
pub async fn delete_propagation(
    pool: &PgPool,
    tenant_id: Uuid,
    prop_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM yeast_propagations WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(prop_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}
