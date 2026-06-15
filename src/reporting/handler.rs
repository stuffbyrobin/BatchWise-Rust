//! Reporting HTTP handlers and router.
//!
//! Port of the Go `internal/reporting/handler_*.go`. The Go module mounts three
//! separate chi routers (`/cost-rates`, `/batch-costs`, `/cost-reports`); here
//! they are combined into one router mounted by the orchestrator at
//! `/api/v1/reporting`. All routes require auth. The tier `FeatureGate` is added
//! by the orchestrator, not here.
//!
//! Note: the Go code defines no standalone duty-calculation endpoint — the duty
//! estimate is computed internally as part of `POST /batch-costs/compute`.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    BatchCostFilter, ComputeBatchCostRequest, CreateRateRequest, GenerateReportRequest,
    PatchRateRequest, RateFilter, ReportFilter,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the reporting router (mounted at `/reporting`), gated by auth + the
/// `reporting` feature flag (tier gate).
pub fn routes(state: AppState) -> Router {
    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "reporting", req, next).await }
    });
    Router::new()
        // cost rates
        .route("/cost-rates", post(create_rate).get(list_rates))
        .route(
            "/cost-rates/{id}",
            get(get_rate)
                .put(replace_rate)
                .patch(patch_rate)
                .delete(delete_rate),
        )
        // batch costs
        .route("/batch-costs", get(list_batch_costs))
        .route("/batch-costs/compute", post(compute_batch_cost))
        .route("/batch-costs/{batch_id}", get(get_batch_cost))
        // cost reports
        .route("/cost-reports", get(list_reports))
        .route("/cost-reports/generate", post(generate_report))
        .route("/cost-reports/{id}", get(get_report).delete(delete_report))
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

// ---- query params ----

#[derive(Debug, Deserialize)]
struct RateListQuery {
    rate_type: Option<String>,
    effective_on: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BatchCostListQuery {
    batch_id: Option<Uuid>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ReportListQuery {
    report_type: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

// ---- cost rates ----

async fn create_rate(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateRateRequest>,
) -> Result<Response, ApiError> {
    let rate = service::create_rate(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/cost-rates/{}", rate.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(rate),
    )
        .into_response())
}

async fn list_rates(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<RateListQuery>,
) -> Result<Response, ApiError> {
    let filter = RateFilter {
        rate_type: q.rate_type,
        effective_on: q.effective_on,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::list_rates(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn get_rate(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let rate = service::get_rate(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(rate).into_response())
}

async fn replace_rate(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateRateRequest>,
) -> Result<Response, ApiError> {
    let rate = service::replace_rate(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(rate).into_response())
}

async fn patch_rate(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchRateRequest>,
) -> Result<Response, ApiError> {
    let rate = service::patch_rate(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(rate).into_response())
}

async fn delete_rate(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_rate(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- batch costs ----

async fn list_batch_costs(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<BatchCostListQuery>,
) -> Result<Response, ApiError> {
    let filter = BatchCostFilter {
        batch_id: q.batch_id,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::list_batch_costs(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn get_batch_cost(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(batch_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let bc = service::get_batch_cost_by_batch_id(&state, ctx.tenant_id()?, batch_id).await?;
    Ok(Json(bc).into_response())
}

async fn compute_batch_cost(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<ComputeBatchCostRequest>,
) -> Result<Response, ApiError> {
    // Go always upserts and returns 200 (on both first compute and re-compute).
    let bc = service::compute_batch_cost(&state, ctx.tenant_id()?, req).await?;
    Ok(Json(bc).into_response())
}

// ---- cost reports ----

async fn list_reports(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ReportListQuery>,
) -> Result<Response, ApiError> {
    let filter = ReportFilter {
        report_type: q.report_type,
        from_date: q.from_date,
        to_date: q.to_date,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::list_reports(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn generate_report(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<GenerateReportRequest>,
) -> Result<Response, ApiError> {
    let rep = service::generate_report(&state, ctx.tenant_id()?, req).await?;
    Ok((StatusCode::CREATED, Json(rep)).into_response())
}

async fn get_report(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let rep = service::get_report(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(rep).into_response())
}

async fn delete_report(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_report(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
