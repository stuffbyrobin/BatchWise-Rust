//! Duty-return business logic: compilation, listing, retrieval, and submission.
//!
//! Port of the Go `internal/compliance/duty/service.go`. Money is `i64` pence.
//! The gross duty is the sum of pre-crystallised `duty_pence` over the period's
//! `duty_events` (each event was costed with [`crate::pkg::duty`] at sale time);
//! Small Producer Relief is applied here via [`crate::pkg::duty::spr_relief_rate`]
//! over the tenant's `sbr_annual_production_hl_pa`.
//!
//! Compiling and submitting a return each record a fire-and-forget compliance
//! audit event.

use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use serde_json::json;
use uuid::Uuid;

use super::models::{CompileRequest, Page, PatchRequest, Return, ReturnFilter};
use super::repository::{self as repo, ReturnWrite};
use crate::audit;
use crate::pkg::duty;
use crate::platform::errors::ApiError;
use crate::state::AppState;

/// Rounds half away from zero, matching Go's `math.Round`.
fn round_half_away(x: f64) -> i64 {
    x.round() as i64
}

/// Compiles (and upserts) a duty return for the requested calendar period.
pub async fn compile_return(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    req: CompileRequest,
) -> Result<Return, ApiError> {
    let from_date = NaiveDate::parse_from_str(&req.period_start, "%Y-%m-%d")
        .map_err(|_| ApiError::validation("period_start", "must be a valid date (YYYY-MM-DD)"))?;
    let to_date = NaiveDate::parse_from_str(&req.period_end, "%Y-%m-%d")
        .map_err(|_| ApiError::validation("period_end", "must be a valid date (YYYY-MM-DD)"))?;

    let from = from_date.and_time(NaiveTime::MIN).and_utc();
    // Extend `to` to end of day so crystallised_at timestamps on that date are
    // included (Go adds 23h59m59s).
    let to = to_date.and_time(NaiveTime::MIN).and_utc()
        + Duration::hours(23)
        + Duration::minutes(59)
        + Duration::seconds(59);

    if to <= from {
        return Err(ApiError::validation(
            "period_end",
            "must be on or after period_start",
        ));
    }

    let overlap = repo::has_submitted_overlap(&state.pool, tenant_id, from, to).await?;
    if overlap {
        return Err(ApiError::business_rule(
            "submitted_period_overlap",
            "period overlaps an already-submitted duty return",
            Default::default(),
        ));
    }

    let summary = repo::sum_duty_events_for_period(&state.pool, tenant_id, from, to).await?;

    let annual_hl_pa = repo::get_tenant_sbr_production(&state.pool, tenant_id)
        .await?
        .ok_or_else(|| ApiError::not_found("tenant"))?;

    let rate = duty::spr_relief_rate(annual_hl_pa);
    let relief_pence = round_half_away(summary.gross_duty_pence as f64 * rate);

    let w = ReturnWrite {
        period_start: req.period_start.clone(),
        period_end: req.period_end.clone(),
        event_count: summary.event_count,
        total_volume_liters: summary.total_volume_liters,
        gross_duty_pence: summary.gross_duty_pence,
        sbr_annual_production_hl_pa: annual_hl_pa,
        sbr_relief_rate_pct: rate * 100.0,
        sbr_relief_pence: relief_pence,
        net_duty_pence: summary.gross_duty_pence - relief_pence,
    };

    let ret = repo::upsert_return(&state.pool, tenant_id, &w).await?;

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_DUTY_COMPILED,
            entity_type: "duty_return",
            entity_id: Some(ret.id),
            actor_user_id: actor_id,
            event_data: json!({
                "period_start": req.period_start,
                "period_end": req.period_end,
                "gross_duty_pence": ret.gross_duty_pence,
                "net_duty_pence": ret.net_duty_pence,
            }),
        },
    )
    .await;
    Ok(ret)
}

/// Lists duty returns.
pub async fn list_returns(
    state: &AppState,
    tenant_id: Uuid,
    filter: ReturnFilter,
) -> Result<Page<Return>, ApiError> {
    Ok(repo::select_returns(&state.pool, tenant_id, &filter).await?)
}

/// Fetches a duty return, 404 if absent or cross-tenant.
pub async fn get_return(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Return, ApiError> {
    repo::select_return_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("duty_return"))
}

/// Marks a draft duty return as submitted. Only `status == "submitted"` is
/// accepted; resubmission and empty returns are rejected.
pub async fn patch_return(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    actor_id: Option<Uuid>,
    req: PatchRequest,
) -> Result<Return, ApiError> {
    if req.status.as_deref() != Some("submitted") {
        return Err(ApiError::validation(
            "status",
            r#"only "submitted" is accepted"#,
        ));
    }

    let ret = repo::select_return_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("duty_return"))?;
    if ret.status == "submitted" {
        return Err(ApiError::business_rule(
            "already_submitted",
            "duty return has already been submitted",
            Default::default(),
        ));
    }
    if ret.event_count == 0 {
        return Err(ApiError::business_rule(
            "no_events",
            "cannot submit a duty return with no events",
            Default::default(),
        ));
    }

    let now = Utc::now();
    repo::update_return_status(&state.pool, tenant_id, id, "submitted", Some(now)).await?;

    let updated = repo::select_return_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("duty_return"))?;

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_DUTY_SUBMITTED,
            entity_type: "duty_return",
            entity_id: Some(id),
            actor_user_id: actor_id,
            event_data: json!({
                "period_start": updated.period_start,
                "period_end": updated.period_end,
                "net_duty_pence": updated.net_duty_pence,
            }),
        },
    )
    .await;
    Ok(updated)
}
