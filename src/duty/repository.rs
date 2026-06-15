//! Data access for duty returns.
//!
//! Port of the Go `internal/compliance/duty/repository.go`. `NUMERIC` columns
//! are selected as `float8`; `DATE` columns (`period_start`, `period_end`) are
//! rendered with `to_char(..., 'YYYY-MM-DD')`. Money is `i64` pence. All domain
//! queries are tenant-scoped.

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Page, Return, ReturnFilter};

const RETURN_COLS: &str = "id, tenant_id, \
    to_char(period_start, 'YYYY-MM-DD') AS period_start, \
    to_char(period_end, 'YYYY-MM-DD') AS period_end, status, \
    event_count, total_volume_liters::float8 AS total_volume_liters, gross_duty_pence, \
    sbr_annual_production_hl_pa::float8 AS sbr_annual_production_hl_pa, \
    sbr_relief_rate_pct::float8 AS sbr_relief_rate_pct, sbr_relief_pence, \
    net_duty_pence, submitted_at, created_at, updated_at";

fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if !(1..=100).contains(&page_size) {
        20
    } else {
        page_size
    };
    (page, page_size)
}

/// The aggregate from `duty_events` for a period.
pub struct EventSummary {
    pub event_count: i32,
    pub total_volume_liters: f64,
    pub gross_duty_pence: i64,
}

/// Aggregates `duty_events` in `[from, to]` where `abv_pct < 8.5`.
pub async fn sum_duty_events_for_period(
    pool: &PgPool,
    tenant_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<EventSummary, sqlx::Error> {
    let row: (i32, f64, i64) = sqlx::query_as(
        "SELECT \
            COUNT(*)::int, \
            COALESCE(SUM(volume_liters), 0)::float8, \
            COALESCE(SUM(duty_pence), 0)::bigint \
         FROM duty_events \
         WHERE tenant_id = $1 \
           AND crystallised_at >= $2 \
           AND crystallised_at <= $3 \
           AND abv_pct < 8.5",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    Ok(EventSummary {
        event_count: row.0,
        total_volume_liters: row.1,
        gross_duty_pence: row.2,
    })
}

/// Returns the tenant's annual production in hLPA used for Small Producer Relief.
pub async fn get_tenant_sbr_production(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Option<f64>, sqlx::Error> {
    sqlx::query_scalar::<_, f64>(
        "SELECT sbr_annual_production_hl_pa::float8 FROM tenants WHERE id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
}

/// Returns true if any submitted return's period overlaps `[from, to]`.
pub async fn has_submitted_overlap(
    pool: &PgPool,
    tenant_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS ( \
            SELECT 1 FROM duty_returns \
            WHERE tenant_id = $1 \
              AND status = 'submitted' \
              AND period_start <= $3 \
              AND period_end   >= $2 \
        )",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
}

/// Scalar inputs for upserting a duty return. `status` is forced to `'draft'` on
/// insert; the id/timestamps are returned by the DB.
pub struct ReturnWrite {
    pub period_start: String,
    pub period_end: String,
    pub event_count: i32,
    pub total_volume_liters: f64,
    pub gross_duty_pence: i64,
    pub sbr_annual_production_hl_pa: f64,
    pub sbr_relief_rate_pct: f64,
    pub sbr_relief_pence: i64,
    pub net_duty_pence: i64,
}

/// Inserts or updates (on `(tenant_id, period_start)`) a duty return and returns
/// the persisted row.
pub async fn upsert_return(
    pool: &PgPool,
    tenant_id: Uuid,
    w: &ReturnWrite,
) -> Result<Return, sqlx::Error> {
    let sql = format!(
        "INSERT INTO duty_returns \
            (tenant_id, period_start, period_end, status, \
             event_count, total_volume_liters, gross_duty_pence, \
             sbr_annual_production_hl_pa, sbr_relief_rate_pct, sbr_relief_pence, \
             net_duty_pence) \
         VALUES ($1,$2::date,$3::date,'draft',$4,$5,$6,$7,$8,$9,$10) \
         ON CONFLICT (tenant_id, period_start) DO UPDATE \
            SET event_count                = EXCLUDED.event_count, \
                total_volume_liters        = EXCLUDED.total_volume_liters, \
                gross_duty_pence           = EXCLUDED.gross_duty_pence, \
                sbr_annual_production_hl_pa = EXCLUDED.sbr_annual_production_hl_pa, \
                sbr_relief_rate_pct        = EXCLUDED.sbr_relief_rate_pct, \
                sbr_relief_pence           = EXCLUDED.sbr_relief_pence, \
                net_duty_pence             = EXCLUDED.net_duty_pence, \
                updated_at                 = now() \
         RETURNING {RETURN_COLS}"
    );
    sqlx::query_as::<_, Return>(&sql)
        .bind(tenant_id)
        .bind(&w.period_start)
        .bind(&w.period_end)
        .bind(w.event_count)
        .bind(w.total_volume_liters)
        .bind(w.gross_duty_pence)
        .bind(w.sbr_annual_production_hl_pa)
        .bind(w.sbr_relief_rate_pct)
        .bind(w.sbr_relief_pence)
        .bind(w.net_duty_pence)
        .fetch_one(pool)
        .await
}

/// Fetches a duty return by id, tenant-scoped.
pub async fn select_return_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Return>, sqlx::Error> {
    let sql = format!("SELECT {RETURN_COLS} FROM duty_returns WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Return>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists duty returns with filters, sorting, and pagination.
pub async fn select_returns(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ReturnFilter,
) -> Result<Page<Return>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(s) = &filter.status {
            if !s.is_empty() {
                qb.push(" AND status = ").push_bind(s.clone());
            }
        }
        if let Some(f) = &filter.from_date {
            if !f.is_empty() {
                qb.push(" AND period_start >= ").push_bind(f.clone());
            }
        }
        if let Some(t) = &filter.to_date {
            if !t.is_empty() {
                qb.push(" AND period_start <= ").push_bind(t.clone());
            }
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM duty_returns");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = match filter.sort.as_deref() {
        Some("period_start") => "period_start ASC",
        Some("-period_start") => "period_start DESC",
        Some("created_at") => "created_at ASC",
        Some("-created_at") => "created_at DESC",
        _ => "period_start DESC",
    };

    let mut list_qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {RETURN_COLS} FROM duty_returns"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb.build_query_as::<Return>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Updates a duty return's status and `submitted_at`, tenant-scoped.
pub async fn update_return_status<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    status: &str,
    submitted_at: Option<DateTime<Utc>>,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE duty_returns SET status=$1, submitted_at=$2, updated_at=now() \
         WHERE tenant_id=$3 AND id=$4",
    )
    .bind(status)
    .bind(submitted_at)
    .bind(tenant_id)
    .bind(id)
    .execute(exec)
    .await?;
    Ok(r.rows_affected() > 0)
}
