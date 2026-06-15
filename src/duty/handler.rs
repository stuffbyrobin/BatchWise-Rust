//! Duty-return HTTP handlers and router.
//!
//! Port of the Go `internal/compliance/duty/handler.go`. The Go module mounts a
//! single chi router at `/duty-returns`; the orchestrator nests this router at
//! that path. All routes require auth and the `duty` feature flag (tier gate).
//!
//! Every endpoint renders JSON with `200 OK` (matching the Go handler, which
//! uses `web.RenderJSON(w, http.StatusOK, ...)` for compile, list, get, and
//! patch alike — there is no `201 Created`).

use axum::extract::{Path, Query, State};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{CompileRequest, PatchRequest, ReturnFilter};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the duty-returns router (mounted at `/duty-returns`), gated by auth +
/// the `duty` feature flag (tier gate).
pub fn routes(state: AppState) -> Router {
    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "duty", req, next).await }
    });
    Router::new()
        .route("/compile", post(compile))
        .route("/", get(list))
        .route("/{id}", get(get_by_id).patch(patch))
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    status: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn compile(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CompileRequest>,
) -> Result<Response, ApiError> {
    let ret = service::compile_return(&state, ctx.tenant_id()?, ctx.actor_id, req).await?;
    Ok(Json(ret).into_response())
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let filter = ReturnFilter {
        status: q.status,
        from_date: q.from_date,
        to_date: q.to_date,
        sort: q.sort,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::list_returns(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn get_by_id(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let ret = service::get_return(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(ret).into_response())
}

async fn patch(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchRequest>,
) -> Result<Response, ApiError> {
    let ret = service::patch_return(&state, ctx.tenant_id()?, id, ctx.actor_id, req).await?;
    Ok(Json(ret).into_response())
}
