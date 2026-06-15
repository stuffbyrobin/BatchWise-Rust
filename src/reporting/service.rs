//! Reporting business logic: cost rate CRUD, batch cost computation, and cost
//! report generation.
//!
//! Port of the Go `internal/reporting/service.go`. Money is `i64` pence.
//! `total_cost_pence` is a DB-generated column; the service reads it back after
//! the upsert. The duty estimate is delegated to [`crate::pkg::duty`].

use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use serde_json::json;
use uuid::Uuid;

use super::models::{
    BatchCost, BatchCostFilter, ComputeBatchCostRequest, CreateRateRequest, GenerateReportRequest,
    Page, PatchRateRequest, ProfitabilityRow, Rate, RateFilter, Report, ReportFilter,
};
use super::repository::{self as repo, BatchCostWrite, NewRate, NewReport};
use crate::batch::models::ListFilter as BatchListFilter;
use crate::batch::service as batch_svc;
use crate::inventory::models::ListFilter as InventoryListFilter;
use crate::inventory::service as inventory_svc;
use crate::pkg::duty;
use crate::pkg::gravity;
use crate::platform::errors::ApiError;
use crate::recipe::service as recipe_svc;
use crate::state::AppState;
use crate::tenant::repository as tenant_repo;

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

/// Maps a `rate_type` to its allowed `unit` values (mirrors Go `allowedUnits`).
fn allowed_units(rate_type: &str) -> Option<&'static [&'static str]> {
    match rate_type {
        "energy" => Some(&["pence_per_kwh"]),
        "labor" => Some(&["pence_per_hour"]),
        "water" => Some(&["pence_per_l"]),
        "duty" => Some(&["pence_per_l_per_pct_abv"]),
        "overhead" => Some(&["pence_per_batch", "percent_of_revenue"]),
        _ => None,
    }
}

fn validate_unit(rate_type: &str, unit: &str) -> Result<(), ApiError> {
    match allowed_units(rate_type) {
        None => Ok(()),
        Some(allowed) if allowed.contains(&unit) => Ok(()),
        Some(allowed) => Err(ApiError::validation(
            "unit",
            &format!(
                "unit {unit:?} is not valid for rate_type {rate_type:?}; allowed: {}",
                allowed.join(", ")
            ),
        )),
    }
}

fn currency_or_default(c: &Option<String>) -> String {
    match c {
        Some(s) if !s.is_empty() => s.clone(),
        _ => "GBP".to_string(),
    }
}

// ---- cost rates ----

/// Creates a cost rate.
pub async fn create_rate(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateRateRequest,
) -> Result<Rate, ApiError> {
    validate_unit(&req.rate_type, &req.unit)?;
    let w = NewRate {
        rate_type: req.rate_type,
        rate_name: req.rate_name,
        unit: req.unit,
        rate_value: req.rate_value,
        currency: currency_or_default(&req.currency),
        effective_from: req.effective_from,
        effective_to: req.effective_to,
        notes: req.notes,
    };
    match repo::insert_rate(&state.pool, tenant_id, &w).await {
        Ok(r) => Ok(r),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "cost_rate",
            "A rate with this type, name, and effective_from already exists.",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Lists cost rates.
pub async fn list_rates(
    state: &AppState,
    tenant_id: Uuid,
    filter: RateFilter,
) -> Result<Page<Rate>, ApiError> {
    Ok(repo::select_rates(&state.pool, tenant_id, &filter).await?)
}

/// Fetches a cost rate, 404 if absent or cross-tenant.
pub async fn get_rate(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Rate, ApiError> {
    repo::select_rate_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cost_rate"))
}

/// Replaces a cost rate (PUT).
pub async fn replace_rate(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: CreateRateRequest,
) -> Result<Rate, ApiError> {
    validate_unit(&req.rate_type, &req.unit)?;
    // Mirror the Go ordering: existence check (404) before the update.
    get_rate(state, tenant_id, id).await?;
    let w = NewRate {
        rate_type: req.rate_type,
        rate_name: req.rate_name,
        unit: req.unit,
        rate_value: req.rate_value,
        currency: currency_or_default(&req.currency),
        effective_from: req.effective_from,
        effective_to: req.effective_to,
        notes: req.notes,
    };
    match repo::update_rate(&state.pool, tenant_id, id, &w).await {
        Ok(Some(r)) => Ok(r),
        Ok(None) => Err(ApiError::not_found("cost_rate")),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "cost_rate",
            "A rate with this type, name, and effective_from already exists.",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Partially updates a cost rate (PATCH).
pub async fn patch_rate(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchRateRequest,
) -> Result<Rate, ApiError> {
    let existing = get_rate(state, tenant_id, id).await?;

    let mut w = NewRate {
        rate_type: existing.rate_type.clone(),
        rate_name: existing.rate_name,
        unit: existing.unit,
        rate_value: existing.rate_value,
        currency: existing.currency,
        effective_from: existing.effective_from,
        effective_to: existing.effective_to,
        notes: existing.notes,
    };
    if let Some(v) = req.rate_name {
        w.rate_name = v;
    }
    if let Some(v) = req.unit {
        w.unit = v;
    }
    if let Some(v) = req.rate_value {
        w.rate_value = v;
    }
    if let Some(v) = req.currency {
        w.currency = v;
    }
    if let Some(v) = req.effective_from {
        w.effective_from = v;
    }
    if req.effective_to.is_some() {
        w.effective_to = req.effective_to;
    }
    if req.notes.is_some() {
        w.notes = req.notes;
    }

    validate_unit(&w.rate_type, &w.unit)?;

    match repo::update_rate(&state.pool, tenant_id, id, &w).await {
        Ok(Some(r)) => Ok(r),
        Ok(None) => Err(ApiError::not_found("cost_rate")),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "cost_rate",
            "A rate with this type, name, and effective_from already exists.",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Deletes a cost rate.
pub async fn delete_rate(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    get_rate(state, tenant_id, id).await?;
    repo::delete_rate(&state.pool, tenant_id, id).await?;
    Ok(())
}

// ---- batch costs ----

/// Rounds half away from zero, matching Go's `math.Round`.
fn round_half_away(x: f64) -> i64 {
    x.round() as i64
}

/// Computes (and upserts) the cost breakdown for a batch.
pub async fn compute_batch_cost(
    state: &AppState,
    tenant_id: Uuid,
    req: ComputeBatchCostRequest,
) -> Result<BatchCost, ApiError> {
    let b = batch_svc::get(state, tenant_id, req.batch_id).await?;

    let tn = tenant_repo::get_by_id(&state.pool, tenant_id)
        .await?
        .ok_or_else(|| ApiError::not_found("tenant"))?;

    let ingredient_cost = repo::sum_ingredient_cost_for_batch(&state.pool, req.batch_id).await?;

    let mut energy_cost = 0i64;
    if let Some(kwh) = req.energy_kwh {
        if kwh > 0.0 {
            if let Some(rate) =
                repo::select_current_rate_for_type(&state.pool, tenant_id, "energy").await?
            {
                energy_cost = round_half_away(kwh * rate.rate_value);
            }
        }
    }

    let mut labor_cost = 0i64;
    if let Some(hours) = req.labor_hours {
        if hours > 0.0 {
            if let Some(rate) =
                repo::select_current_rate_for_type(&state.pool, tenant_id, "labor").await?
            {
                labor_cost = round_half_away(hours * rate.rate_value);
            }
        }
    }

    let mut water_cost = 0i64;
    if let Some(liters) = req.water_liters {
        if liters > 0.0 {
            if let Some(rate) =
                repo::select_current_rate_for_type(&state.pool, tenant_id, "water").await?
            {
                water_cost = round_half_away(liters * rate.rate_value);
            }
        }
    }

    let overhead_cost = req.overhead_pence.unwrap_or(0);

    let mut estimated_duty = 0i64;
    if let (Some(vol), Some(og), Some(fg)) = (b.actual_volume_liters, b.actual_og, b.actual_fg) {
        if let Ok(abv) = gravity::calculate_abv(og, fg) {
            // Go's CalculateDuty fails open (logs WARN, returns 0) for
            // unsupported jurisdictions; the Rust helper returns Err, which we
            // treat as zero to preserve that behaviour.
            estimated_duty = duty::calculate_duty(&tn.country, vol, abv).unwrap_or(0);
        }
    }

    let revenue_pence = repo::sum_revenue_for_batch(&state.pool, tenant_id, req.batch_id).await?;

    // total_cost_pence is DB-generated; we compute it here only to derive
    // cost_per_liter_pence prior to the upsert.
    let total_cost =
        ingredient_cost + energy_cost + labor_cost + water_cost + overhead_cost + estimated_duty;
    let cost_per_liter_pence = match b.actual_volume_liters {
        Some(vol) if vol > 0.0 => Some(round_half_away(total_cost as f64 / vol)),
        _ => None,
    };

    let w = BatchCostWrite {
        batch_id: req.batch_id,
        ingredient_cost_pence: ingredient_cost,
        energy_cost_pence: energy_cost,
        labor_cost_pence: labor_cost,
        water_cost_pence: water_cost,
        overhead_cost_pence: overhead_cost,
        estimated_duty_pence: estimated_duty,
        revenue_pence,
        cost_per_liter_pence,
        cost_per_unit_pence: None,
    };

    let bc = repo::upsert_batch_cost(&state.pool, tenant_id, &w).await?;
    Ok(bc)
}

/// Lists batch costs.
pub async fn list_batch_costs(
    state: &AppState,
    tenant_id: Uuid,
    filter: BatchCostFilter,
) -> Result<Page<BatchCost>, ApiError> {
    Ok(repo::select_batch_costs(&state.pool, tenant_id, &filter).await?)
}

/// Fetches a batch cost by batch id, 404 if absent or cross-tenant.
pub async fn get_batch_cost_by_batch_id(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<BatchCost, ApiError> {
    repo::select_batch_cost_by_batch_id(&state.pool, tenant_id, batch_id)
        .await?
        .ok_or_else(|| ApiError::not_found("batch_cost"))
}

// ---- cost reports ----

fn validate_report_request(req: &GenerateReportRequest) -> Result<(), ApiError> {
    match req.report_type.as_str() {
        "batch" if req.batch_id.is_none() => Err(ApiError::validation(
            "batch_id",
            "required for report_type=batch",
        )),
        "recipe" if req.recipe_id.is_none() => Err(ApiError::validation(
            "recipe_id",
            "required for report_type=recipe",
        )),
        "period" if req.period_start.is_none() || req.period_end.is_none() => {
            Err(ApiError::validation(
                "period_start/period_end",
                "both required for report_type=period",
            ))
        }
        "profitability" if req.period_start.is_none() || req.period_end.is_none() => {
            Err(ApiError::validation(
                "period_start/period_end",
                "both required for report_type=profitability",
            ))
        }
        _ => Ok(()),
    }
}

/// Generates and stores a cost report.
pub async fn generate_report(
    state: &AppState,
    tenant_id: Uuid,
    req: GenerateReportRequest,
) -> Result<Report, ApiError> {
    validate_report_request(&req)?;
    let data = build_report_data(state, tenant_id, &req).await?;
    let w = NewReport {
        report_type: req.report_type.clone(),
        period_start: req.period_start.clone(),
        period_end: req.period_end.clone(),
        report_data: data,
    };
    Ok(repo::insert_report(&state.pool, tenant_id, &w).await?)
}

/// Lists cost reports.
pub async fn list_reports(
    state: &AppState,
    tenant_id: Uuid,
    filter: ReportFilter,
) -> Result<Page<Report>, ApiError> {
    Ok(repo::select_reports(&state.pool, tenant_id, &filter).await?)
}

/// Fetches a cost report, 404 if absent or cross-tenant.
pub async fn get_report(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Report, ApiError> {
    repo::select_report_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cost_report"))
}

/// Deletes a cost report.
pub async fn delete_report(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    get_report(state, tenant_id, id).await?;
    repo::delete_report(&state.pool, tenant_id, id).await?;
    Ok(())
}

/// Returns the sum of `estimated_duty_pence` for batch costs computed since
/// `since`.
pub async fn sum_recent_duty_pence(
    state: &AppState,
    tenant_id: Uuid,
    since: chrono::DateTime<Utc>,
) -> Result<i64, ApiError> {
    Ok(repo::sum_recent_duty_pence(&state.pool, tenant_id, since).await?)
}

// ---- report data assembly ----

fn batch_totals(bc: &Option<BatchCost>) -> serde_json::Value {
    match bc {
        None => json!({}),
        Some(bc) => json!({
            "total_cost_pence": bc.total_cost_pence,
            "cost_per_liter_pence": bc.cost_per_liter_pence,
            "estimated_duty_pence": bc.estimated_duty_pence,
        }),
    }
}

async fn build_report_data(
    state: &AppState,
    tenant_id: Uuid,
    req: &GenerateReportRequest,
) -> Result<serde_json::Value, ApiError> {
    match req.report_type.as_str() {
        "batch" => {
            let batch_id = req.batch_id.ok_or_else(|| {
                ApiError::validation("batch_id", "required for report_type=batch")
            })?;
            let b = batch_svc::get(state, tenant_id, batch_id).await?;
            let bc = repo::select_batch_cost_by_batch_id(&state.pool, tenant_id, batch_id).await?;
            Ok(json!({
                "batch": b,
                "breakdown": bc,
                "totals": batch_totals(&bc),
            }))
        }
        "recipe" => {
            let recipe_id = req.recipe_id.ok_or_else(|| {
                ApiError::validation("recipe_id", "required for report_type=recipe")
            })?;
            let rec = recipe_svc::get(state, tenant_id, recipe_id).await?;
            Ok(json!({
                "recipe": rec,
                "batches": [],
                "averages": {},
            }))
        }
        "period" => {
            let batches = batch_svc::list(
                state,
                tenant_id,
                BatchListFilter {
                    page: 1,
                    page_size: 200,
                    ..Default::default()
                },
            )
            .await?;
            Ok(json!({
                "period": { "start": req.period_start, "end": req.period_end },
                "batches": batches.items,
                "totals_by_type": {},
            }))
        }
        "inventory" => {
            let items = inventory_svc::list(
                state,
                tenant_id,
                InventoryListFilter {
                    page: 1,
                    page_size: 500,
                    ..Default::default()
                },
            )
            .await?;
            Ok(json!({
                "on_hand_value_pence": 0,
                "by_type": items.items,
                "expiring_soon": [],
            }))
        }
        "profitability" => {
            let start = req.period_start.as_deref().ok_or_else(|| {
                ApiError::validation(
                    "period_start/period_end",
                    "both required for report_type=profitability",
                )
            })?;
            let end = req.period_end.as_deref().ok_or_else(|| {
                ApiError::validation(
                    "period_start/period_end",
                    "both required for report_type=profitability",
                )
            })?;

            let from_date = NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|_| {
                ApiError::validation("period_start", "must be a valid date in YYYY-MM-DD format")
            })?;
            let to_date = NaiveDate::parse_from_str(end, "%Y-%m-%d").map_err(|_| {
                ApiError::validation("period_end", "must be a valid date in YYYY-MM-DD format")
            })?;

            let from = from_date.and_time(NaiveTime::MIN).and_utc();
            // Make the end date inclusive (Go adds 24h).
            let to = to_date.and_time(NaiveTime::MIN).and_utc() + Duration::hours(24);

            let prof_rows: Vec<ProfitabilityRow> =
                repo::select_profitability_rows(&state.pool, tenant_id, from, to).await?;
            let total_cost: i64 = prof_rows.iter().map(|r| r.total_cost_pence).sum();
            let total_revenue: i64 = prof_rows.iter().map(|r| r.revenue_pence).sum();

            Ok(json!({
                "period": { "start": req.period_start, "end": req.period_end },
                "batches": prof_rows,
                "total_cost_pence": total_cost,
                "total_revenue_pence": total_revenue,
                "total_margin_pence": total_revenue - total_cost,
            }))
        }
        _ => Err(ApiError::validation("report_type", "unknown")),
    }
}
