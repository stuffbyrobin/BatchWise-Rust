//! Member HTTP handlers and router (`/members`).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch};
use axum::{Json, Router};
use uuid::Uuid;

use super::models::{CreateInvitationRequest, PatchMemberRequest};
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
        .route(
            "/invitations",
            get(list_invitations).post(create_invitation),
        )
        .route("/invitations/{id}", delete(revoke_invitation))
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

async fn list_invitations(
    State(state): State<AppState>,
    ctx: RequestContext,
) -> Result<Response, ApiError> {
    Ok(Json(service::list_invitations(&state, ctx.tenant_id()?).await?).into_response())
}

async fn create_invitation(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateInvitationRequest>,
) -> Result<Response, ApiError> {
    let created = service::invite(&state, ctx.tenant_id()?, ctx.actor_id, ctx.role()?, req).await?;
    Ok((StatusCode::CREATED, Json(created)).into_response())
}

async fn revoke_invitation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::revoke_invitation(&state, ctx.tenant_id()?, ctx.actor_id, ctx.role()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
