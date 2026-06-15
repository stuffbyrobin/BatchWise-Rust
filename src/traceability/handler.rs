//! Traceability HTTP handlers and router.
//!
//! Port of the Go `internal/traceability/handler.go`. The Go module mounts three
//! read-only routes under `/traceability`; here they are combined into one router
//! merged by the orchestrator. All routes require auth and the `traceability`
//! feature flag. The Go fire-and-forget audit write in `RecallScope` is omitted
//! (no audit module yet).

use axum::extract::{Path, Query, State};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::state::AppState;

/// Builds the traceability router (paths under `/traceability`), gated by auth +
/// the `traceability` feature flag (tier gate).
pub fn routes(state: AppState) -> Router {
    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "traceability", req, next).await }
    });
    Router::new()
        .route(
            "/traceability/ingredient-lots/{lot_number}",
            get(trace_ingredient_lot),
        )
        .route(
            "/traceability/packaging-runs/{id}",
            get(trace_packaging_run),
        )
        .route("/traceability/recall", get(recall_scope))
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

/// `?lot_number=` query parameter for the recall-scope endpoint.
#[derive(Debug, Deserialize)]
struct RecallQuery {
    lot_number: Option<String>,
}

/// `GET /traceability/ingredient-lots/{lot_number}` — forward trace.
async fn trace_ingredient_lot(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(lot_number): Path<String>,
) -> Result<Response, ApiError> {
    if lot_number.is_empty() {
        return Err(ApiError::validation("lot_number", "must not be empty"));
    }
    let trace = service::trace_ingredient_lot(&state, ctx.tenant_id()?, &lot_number).await?;
    Ok(Json(trace).into_response())
}

/// `GET /traceability/packaging-runs/{id}` — backward trace.
async fn trace_packaging_run(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let trace = service::trace_packaging_run(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(trace).into_response())
}

/// `GET /traceability/recall?lot_number=` — recall scope.
async fn recall_scope(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<RecallQuery>,
) -> Result<Response, ApiError> {
    let lot_number = q.lot_number.unwrap_or_default();
    if lot_number.is_empty() {
        return Err(ApiError::validation(
            "lot_number",
            "required query parameter",
        ));
    }
    let scope = service::recall_scope(&state, ctx.tenant_id()?, &lot_number).await?;
    Ok(Json(scope).into_response())
}
