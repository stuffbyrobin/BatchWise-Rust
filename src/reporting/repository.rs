//! Data access for cost rates, batch costs, and cost reports.
//!
//! Port of the Go `internal/reporting/repository.go`. `NUMERIC` columns are
//! selected as `float8`; `DATE` columns are rendered with
//! `to_char(..., 'YYYY-MM-DD')`; `report_data` is `JSONB`. `total_cost_pence` is
//! a DB-generated column — it is never inserted or updated, only selected back.

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{
    BatchCost, BatchCostFilter, Page, ProfitabilityRow, Rate, RateFilter, Report, ReportFilter,
};

const RATE_COLS: &str = "id, tenant_id, rate_type, rate_name, unit, \
    rate_value::float8 AS rate_value, currency, \
    to_char(effective_from, 'YYYY-MM-DD') AS effective_from, \
    to_char(effective_to, 'YYYY-MM-DD') AS effective_to, \
    notes, created_at, updated_at";

const BC_COLS: &str = "id, tenant_id, batch_id, ingredient_cost_pence, energy_cost_pence, \
    labor_cost_pence, water_cost_pence, overhead_cost_pence, estimated_duty_pence, \
    total_cost_pence, revenue_pence, \
    (revenue_pence - total_cost_pence) AS margin_pence, \
    cost_per_liter_pence, cost_per_unit_pence, computed_at";

const REPORT_COLS: &str = "id, tenant_id, report_type, \
    to_char(period_start, 'YYYY-MM-DD') AS period_start, \
    to_char(period_end, 'YYYY-MM-DD') AS period_end, \
    report_data, generated_at";

fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 { 20 } else { page_size };
    (page, page_size)
}

// ---- cost rates ----

/// Values for inserting a new cost rate.
pub struct NewRate {
    pub rate_type: String,
    pub rate_name: String,
    pub unit: String,
    pub rate_value: f64,
    pub currency: String,
    pub effective_from: String,
    pub effective_to: Option<String>,
    pub notes: Option<String>,
}

/// Inserts a cost rate and returns the created row.
pub async fn insert_rate<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    r: &NewRate,
) -> Result<Rate, sqlx::Error> {
    let sql = format!(
        "INSERT INTO cost_rates \
         (tenant_id, rate_type, rate_name, unit, rate_value, currency, effective_from, effective_to, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7::date,$8::date,$9) RETURNING {RATE_COLS}"
    );
    sqlx::query_as::<_, Rate>(&sql)
        .bind(tenant_id)
        .bind(&r.rate_type)
        .bind(&r.rate_name)
        .bind(&r.unit)
        .bind(r.rate_value)
        .bind(&r.currency)
        .bind(&r.effective_from)
        .bind(&r.effective_to)
        .bind(&r.notes)
        .fetch_one(exec)
        .await
}

/// Fetches a cost rate by id, tenant-scoped.
pub async fn select_rate_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Rate>, sqlx::Error> {
    let sql = format!("SELECT {RATE_COLS} FROM cost_rates WHERE id=$1 AND tenant_id=$2");
    sqlx::query_as::<_, Rate>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Lists cost rates with filters and pagination.
pub async fn select_rates(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &RateFilter,
) -> Result<Page<Rate>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(t) = &filter.rate_type {
            qb.push(" AND rate_type = ").push_bind(t.clone());
        }
        if let Some(on) = &filter.effective_on {
            qb.push(" AND effective_from <= ")
                .push_bind(on.clone())
                .push(" AND (effective_to IS NULL OR effective_to >= ")
                .push_bind(on.clone())
                .push(")");
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM cost_rates");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb = QueryBuilder::<Postgres>::new(format!("SELECT {RATE_COLS} FROM cost_rates"));
    push_where(&mut list_qb);
    list_qb.push(" ORDER BY effective_from DESC, rate_type ");
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb.build_query_as::<Rate>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Replaces all mutable columns of a cost rate; returns the new row or `None`.
pub async fn update_rate<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    r: &NewRate,
) -> Result<Option<Rate>, sqlx::Error> {
    let sql = format!(
        "UPDATE cost_rates SET rate_type=$3, rate_name=$4, unit=$5, rate_value=$6, currency=$7, \
         effective_from=$8::date, effective_to=$9::date, notes=$10, updated_at=now() \
         WHERE id=$1 AND tenant_id=$2 RETURNING {RATE_COLS}"
    );
    sqlx::query_as::<_, Rate>(&sql)
        .bind(id)
        .bind(tenant_id)
        .bind(&r.rate_type)
        .bind(&r.rate_name)
        .bind(&r.unit)
        .bind(r.rate_value)
        .bind(&r.currency)
        .bind(&r.effective_from)
        .bind(&r.effective_to)
        .bind(&r.notes)
        .fetch_optional(exec)
        .await
}

/// Deletes a cost rate; returns true if a row was removed.
pub async fn delete_rate<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM cost_rates WHERE id=$1 AND tenant_id=$2")
        .bind(id)
        .bind(tenant_id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Returns the rate of a given type currently in effect (latest effective_from
/// covering today), or `None`.
pub async fn select_current_rate_for_type(
    pool: &PgPool,
    tenant_id: Uuid,
    rate_type: &str,
) -> Result<Option<Rate>, sqlx::Error> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let sql = format!(
        "SELECT {RATE_COLS} FROM cost_rates \
         WHERE tenant_id=$1 AND rate_type=$2 \
           AND effective_from <= $3 \
           AND (effective_to IS NULL OR effective_to >= $3) \
         ORDER BY effective_from DESC LIMIT 1"
    );
    sqlx::query_as::<_, Rate>(&sql)
        .bind(tenant_id)
        .bind(rate_type)
        .bind(today)
        .fetch_optional(pool)
        .await
}

// ---- batch costs ----

/// Sums ingredient costs (in pence) recorded against a batch.
///
/// `batch_ingredients` has no `tenant_id`; tenant scoping is enforced via the
/// batch FK at the call site.
pub async fn sum_ingredient_cost_for_batch(
    pool: &PgPool,
    batch_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(cost_pence),0)::bigint FROM batch_ingredients WHERE batch_id=$1",
    )
    .bind(batch_id)
    .fetch_one(pool)
    .await
}

/// Scalar inputs for upserting a batch cost. `total_cost_pence` is omitted: it is
/// a DB-generated column.
pub struct BatchCostWrite {
    pub batch_id: Uuid,
    pub ingredient_cost_pence: i64,
    pub energy_cost_pence: i64,
    pub labor_cost_pence: i64,
    pub water_cost_pence: i64,
    pub overhead_cost_pence: i64,
    pub estimated_duty_pence: i64,
    pub revenue_pence: i64,
    pub cost_per_liter_pence: Option<i64>,
    pub cost_per_unit_pence: Option<i64>,
}

/// Inserts or updates a batch cost row and returns the persisted row (with the
/// generated `total_cost_pence` and derived `margin_pence`).
pub async fn upsert_batch_cost(
    pool: &PgPool,
    tenant_id: Uuid,
    w: &BatchCostWrite,
) -> Result<BatchCost, sqlx::Error> {
    let sql = format!(
        "INSERT INTO batch_costs \
         (tenant_id, batch_id, ingredient_cost_pence, energy_cost_pence, labor_cost_pence, \
          water_cost_pence, overhead_cost_pence, estimated_duty_pence, revenue_pence, \
          cost_per_liter_pence, cost_per_unit_pence, computed_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,now()) \
         ON CONFLICT (tenant_id, batch_id) DO UPDATE SET \
            ingredient_cost_pence = EXCLUDED.ingredient_cost_pence, \
            energy_cost_pence     = EXCLUDED.energy_cost_pence, \
            labor_cost_pence      = EXCLUDED.labor_cost_pence, \
            water_cost_pence      = EXCLUDED.water_cost_pence, \
            overhead_cost_pence   = EXCLUDED.overhead_cost_pence, \
            estimated_duty_pence  = EXCLUDED.estimated_duty_pence, \
            revenue_pence         = EXCLUDED.revenue_pence, \
            cost_per_liter_pence  = EXCLUDED.cost_per_liter_pence, \
            cost_per_unit_pence   = EXCLUDED.cost_per_unit_pence, \
            computed_at           = now() \
         RETURNING {BC_COLS}"
    );
    sqlx::query_as::<_, BatchCost>(&sql)
        .bind(tenant_id)
        .bind(w.batch_id)
        .bind(w.ingredient_cost_pence)
        .bind(w.energy_cost_pence)
        .bind(w.labor_cost_pence)
        .bind(w.water_cost_pence)
        .bind(w.overhead_cost_pence)
        .bind(w.estimated_duty_pence)
        .bind(w.revenue_pence)
        .bind(w.cost_per_liter_pence)
        .bind(w.cost_per_unit_pence)
        .fetch_one(pool)
        .await
}

/// Fetches a batch cost by batch id, tenant-scoped.
pub async fn select_batch_cost_by_batch_id(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<Option<BatchCost>, sqlx::Error> {
    let sql = format!("SELECT {BC_COLS} FROM batch_costs WHERE batch_id=$1 AND tenant_id=$2");
    sqlx::query_as::<_, BatchCost>(&sql)
        .bind(batch_id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Lists batch costs with filters and pagination.
pub async fn select_batch_costs(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &BatchCostFilter,
) -> Result<Page<BatchCost>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(b) = filter.batch_id {
            qb.push(" AND batch_id = ").push_bind(b);
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM batch_costs");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb = QueryBuilder::<Postgres>::new(format!("SELECT {BC_COLS} FROM batch_costs"));
    push_where(&mut list_qb);
    list_qb.push(" ORDER BY computed_at DESC ");
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb
        .build_query_as::<BatchCost>()
        .fetch_all(pool)
        .await?;

    Ok(Page::new(items, total, page, page_size))
}

// ---- cost reports ----

/// Values for inserting a cost report.
pub struct NewReport {
    pub report_type: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub report_data: serde_json::Value,
}

/// Inserts a cost report and returns the created row.
pub async fn insert_report<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    r: &NewReport,
) -> Result<Report, sqlx::Error> {
    let sql = format!(
        "INSERT INTO cost_reports (tenant_id, report_type, period_start, period_end, report_data) \
         VALUES ($1,$2,$3::date,$4::date,$5::jsonb) RETURNING {REPORT_COLS}"
    );
    sqlx::query_as::<_, Report>(&sql)
        .bind(tenant_id)
        .bind(&r.report_type)
        .bind(&r.period_start)
        .bind(&r.period_end)
        .bind(sqlx::types::Json(&r.report_data))
        .fetch_one(exec)
        .await
}

/// Fetches a cost report by id, tenant-scoped.
pub async fn select_report_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Report>, sqlx::Error> {
    let sql = format!("SELECT {REPORT_COLS} FROM cost_reports WHERE id=$1 AND tenant_id=$2");
    sqlx::query_as::<_, Report>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Lists cost reports with filters and pagination.
pub async fn select_reports(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ReportFilter,
) -> Result<Page<Report>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(t) = &filter.report_type {
            qb.push(" AND report_type = ").push_bind(t.clone());
        }
        if let Some(f) = &filter.from_date {
            qb.push(" AND generated_at >= ").push_bind(f.clone());
        }
        if let Some(t) = &filter.to_date {
            qb.push(" AND generated_at <= ").push_bind(t.clone());
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM cost_reports");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {REPORT_COLS} FROM cost_reports"));
    push_where(&mut list_qb);
    list_qb.push(" ORDER BY generated_at DESC ");
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb.build_query_as::<Report>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Deletes a cost report; returns true if a row was removed.
pub async fn delete_report<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM cost_reports WHERE id=$1 AND tenant_id=$2")
        .bind(id)
        .bind(tenant_id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Sums `estimated_duty_pence` over batch costs computed since `since`.
pub async fn sum_recent_duty_pence(
    pool: &PgPool,
    tenant_id: Uuid,
    since: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        // SUM() over a BIGINT column yields NUMERIC in Postgres; cast back to
        // BIGINT so it decodes into i64 (pence are whole numbers).
        "SELECT COALESCE(SUM(estimated_duty_pence), 0)::bigint FROM batch_costs \
         WHERE tenant_id = $1 AND computed_at >= $2",
    )
    .bind(tenant_id)
    .bind(since)
    .fetch_one(pool)
    .await
}

/// Per-batch profitability rows in `[from, to)`, ordered by batch creation.
pub async fn select_profitability_rows(
    pool: &PgPool,
    tenant_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<ProfitabilityRow>, sqlx::Error> {
    sqlx::query_as::<_, ProfitabilityRow>(
        "SELECT \
            b.id AS batch_id, \
            b.name AS batch_name, \
            b.created_at, \
            bc.total_cost_pence, \
            bc.revenue_pence, \
            (bc.revenue_pence - bc.total_cost_pence) AS margin_pence \
         FROM batch_costs bc \
         JOIN batches b ON b.id = bc.batch_id \
         WHERE bc.tenant_id = $1 AND b.created_at >= $2 AND b.created_at < $3 \
         ORDER BY b.created_at ASC",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
}

/// Total revenue (quantity × unit price) from fulfilled/invoiced order items
/// linked to a batch. Ported from the Go sales module's `SumRevenueForBatch`,
/// kept here because the reporting service is the sole consumer in this port.
pub async fn sum_revenue_for_batch(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(oi.quantity * oi.unit_price_pence), 0)::bigint \
         FROM order_items oi \
         JOIN orders o ON o.id = oi.order_id \
         WHERE oi.batch_id = $1 \
           AND o.tenant_id = $2 \
           AND o.status IN ('fulfilled', 'invoiced')",
    )
    .bind(batch_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await
}
