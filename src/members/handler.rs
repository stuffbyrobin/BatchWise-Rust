//! Member HTTP handlers and router (`/members`).

use axum::extract::{Path, State};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch};
use axum::{Json, Router};
use uuid::Uuid;

use super::models::PatchMemberRequest;
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the members router (mounted at `/members`; Owners and Managers only).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(list))
        .route("/{id}", patch(patch_member))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

async fn list(State(state): State<AppState>, ctx: RequestContext) -> Result<Response, ApiError> {
    Ok(Json(service::list(&state, ctx.tenant_id()?).await?).into_response())
}

async fn patch_member(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchMemberRequest>,
) -> Result<Response, ApiError> {
    let member =
        service::patch(&state, ctx.tenant_id()?, ctx.actor_id, ctx.role()?, id, req).await?;
    Ok(Json(member).into_response())
}
