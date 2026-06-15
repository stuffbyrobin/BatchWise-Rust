//! Data access for container assets and logs.
//!
//! Port of the Go `internal/tracking/repository.go`. `NUMERIC` columns are
//! selected as `float8`; dates render with `to_char` and bind back via `::date`.

use sqlx::{PgConnection, PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Asset, AssetFilter, Log, LogFilter, Page};

const ASSET_COLS: &str = "id, tenant_id, asset_number, container_type, \
    capacity_liters::float8 AS capacity_liters, deposit_pence, status, current_batch_id, \
    current_customer_name, to_char(last_fill_date, 'YYYY-MM-DD') AS last_fill_date, \
    to_char(last_return_date, 'YYYY-MM-DD') AS last_return_date, notes, created_at, updated_at";

const LOG_COLS: &str =
    "id, tenant_id, container_id, event_type, from_status, to_status, batch_id, \
    customer_name, notes, logged_by_user_id, created_at";

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

/// Inserts a new asset (status `empty`) and returns it.
pub async fn insert_asset<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    asset_number: &str,
    container_type: &str,
    capacity_liters: f64,
    deposit_pence: i64,
    notes: Option<&str>,
) -> Result<Asset, sqlx::Error> {
    let sql = format!(
        "INSERT INTO container_assets (tenant_id, asset_number, container_type, capacity_liters, \
         deposit_pence, status, notes) VALUES ($1,$2,$3,$4,$5,'empty',$6) RETURNING {ASSET_COLS}"
    );
    sqlx::query_as::<_, Asset>(&sql)
        .bind(tenant_id)
        .bind(asset_number)
        .bind(container_type)
        .bind(capacity_liters)
        .bind(deposit_pence)
        .bind(notes)
        .fetch_one(exec)
        .await
}

/// Fetches an asset by id, tenant-scoped.
pub async fn select_asset_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Asset>, sqlx::Error> {
    let sql = format!("SELECT {ASSET_COLS} FROM container_assets WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Asset>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Persists all mutable asset fields (including status / dates / batch / customer).
pub async fn update_asset_full<'e, E: PgExecutor<'e>>(
    exec: E,
    a: &Asset,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE container_assets SET asset_number=$3, container_type=$4, capacity_liters=$5, \
         deposit_pence=$6, status=$7, current_batch_id=$8, current_customer_name=$9, \
         last_fill_date=$10::date, last_return_date=$11::date, notes=$12, updated_at=now() \
         WHERE tenant_id=$1 AND id=$2",
    )
    .bind(a.tenant_id)
    .bind(a.id)
    .bind(&a.asset_number)
    .bind(&a.container_type)
    .bind(a.capacity_liters)
    .bind(a.deposit_pence)
    .bind(&a.status)
    .bind(a.current_batch_id)
    .bind(&a.current_customer_name)
    .bind(&a.last_fill_date)
    .bind(&a.last_return_date)
    .bind(&a.notes)
    .execute(exec)
    .await
    .map(|_| ())
}

/// Deletes an asset; returns true if a row was removed.
pub async fn delete_asset(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM container_assets WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// True if the container has any log entries.
pub async fn has_logs(pool: &PgPool, container_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM container_logs WHERE container_id=$1)",
    )
    .bind(container_id)
    .fetch_one(pool)
    .await
}

/// Lists assets with filters.
pub async fn select_assets(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &AssetFilter,
    order_by: &str,
) -> Result<Page<Asset>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(s) = &filter.status {
            qb.push(" AND status = ").push_bind(s.clone());
        }
        if let Some(t) = &filter.container_type {
            qb.push(" AND container_type = ").push_bind(t.clone());
        }
        if let Some(b) = filter.current_batch_id {
            qb.push(" AND current_batch_id = ").push_bind(b);
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM container_assets");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {ASSET_COLS} FROM container_assets"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by} "));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<Asset>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Inserts a log entry.
pub async fn insert_log(conn: &mut PgConnection, l: &Log) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO container_logs (tenant_id, container_id, event_type, from_status, to_status, \
         batch_id, customer_name, notes, logged_by_user_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(l.tenant_id)
    .bind(l.container_id)
    .bind(&l.event_type)
    .bind(&l.from_status)
    .bind(&l.to_status)
    .bind(l.batch_id)
    .bind(&l.customer_name)
    .bind(&l.notes)
    .bind(l.logged_by_user_id)
    .execute(&mut *conn)
    .await
    .map(|_| ())
}

/// Fetches a log by id, tenant-scoped.
pub async fn select_log_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Log>, sqlx::Error> {
    let sql = format!("SELECT {LOG_COLS} FROM container_logs WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Log>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists logs with filters.
pub async fn select_logs(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &LogFilter,
    order_by: &str,
) -> Result<Page<Log>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(c) = filter.container_id {
            qb.push(" AND container_id = ").push_bind(c);
        }
        if let Some(e) = &filter.event_type {
            qb.push(" AND event_type = ").push_bind(e.clone());
        }
        if let Some(f) = &filter.from_date {
            qb.push(" AND created_at >= ")
                .push_bind(f.clone())
                .push("::date");
        }
        if let Some(t) = &filter.to_date {
            qb.push(" AND created_at < (")
                .push_bind(t.clone())
                .push("::date + 1)");
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM container_logs");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {LOG_COLS} FROM container_logs"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by} "));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<Log>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Counts assets whose status is in `statuses`.
pub async fn count_assets_by_statuses(
    pool: &PgPool,
    tenant_id: Uuid,
    statuses: &[String],
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM container_assets WHERE tenant_id=$1 AND status = ANY($2)",
    )
    .bind(tenant_id)
    .bind(statuses)
    .fetch_one(pool)
    .await
}
