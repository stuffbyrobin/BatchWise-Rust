//! Tenant HTTP handlers and router.
//!
//! Port of the Go `internal/tenant/handler.go`. Both routes require a valid JWT
//! (the `require_auth` layer populates tenant/user ids on the request context).

use axum::extract::State;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use super::models::{Response as TenantResponse, UpdateRequest};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the tenant router (mounted at `/tenants`).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/current", get(get_current).patch(patch_current))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

async fn get_current(
    State(state): State<AppState>,
    ctx: RequestContext,
) -> Result<Response, ApiError> {
    let tenant = service::get_current(&state.pool, ctx.tenant_id()?).await?;
    Ok(Json(TenantResponse::from(tenant)).into_response())
}

async fn patch_current(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<UpdateRequest>,
) -> Result<Response, ApiError> {
    if req.is_empty() {
        return Err(ApiError::validation("body", "at least one field required"));
    }
    let tenant = service::update(&state.pool, ctx.user_id()?, ctx.tenant_id()?, req).await?;
    Ok(Json(TenantResponse::from(tenant)).into_response())
}
