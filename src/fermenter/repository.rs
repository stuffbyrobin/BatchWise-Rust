//! Data access for fermenters. `capacity_liters` is `DOUBLE PRECISION`, so it
//! maps straight to `f64` with no cast.

use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Fermenter, FermenterWrite, ListFilter, Page};

const FERM_COLS: &str = "id, tenant_id, name, capacity_liters, notes, created_at, updated_at";

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

/// Inserts a fermenter and returns the created row.
pub async fn insert<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    w: &FermenterWrite,
) -> Result<Fermenter, sqlx::Error> {
    let sql = format!(
        "INSERT INTO fermenters (tenant_id, name, capacity_liters, notes) \
         VALUES ($1,$2,$3,$4) RETURNING {FERM_COLS}"
    );
    sqlx::query_as::<_, Fermenter>(&sql)
        .bind(tenant_id)
        .bind(&w.name)
        .bind(w.capacity_liters)
        .bind(&w.notes)
        .fetch_one(exec)
        .await
}

/// Updates all scalar columns; returns the new row or `None` if not found.
pub async fn update_full<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    w: &FermenterWrite,
) -> Result<Option<Fermenter>, sqlx::Error> {
    let sql = format!(
        "UPDATE fermenters SET name=$3, capacity_liters=$4, notes=$5, updated_at=now() \
         WHERE tenant_id=$1 AND id=$2 RETURNING {FERM_COLS}"
    );
    sqlx::query_as::<_, Fermenter>(&sql)
        .bind(tenant_id)
        .bind(id)
        .bind(&w.name)
        .bind(w.capacity_liters)
        .bind(&w.notes)
        .fetch_optional(exec)
        .await
}

/// Fetches a fermenter by id, tenant-scoped.
pub async fn select_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Fermenter>, sqlx::Error> {
    let sql = format!("SELECT {FERM_COLS} FROM fermenters WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Fermenter>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists fermenters with filters and a pre-validated `order_by` clause.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
    order_by: &str,
) -> Result<Page<Fermenter>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(n) = &filter.name {
            qb.push(" AND lower(name) LIKE ")
                .push_bind(format!("%{}%", n.to_lowercase()));
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM fermenters");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb = QueryBuilder::<Postgres>::new(format!("SELECT {FERM_COLS} FROM fermenters"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb
        .build_query_as::<Fermenter>()
        .fetch_all(pool)
        .await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Deletes a fermenter; returns true if a row was removed. Assigned batches are
/// detached automatically (`batches.fermenter_id` is `ON DELETE SET NULL`).
pub async fn delete_by_id<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM fermenters WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}
