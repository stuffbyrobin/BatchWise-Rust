//! Auth HTTP handlers and router.
//!
//! Port of the Go `internal/auth/handler.go`. Handlers decode + validate via
//! [`ValidatedJson`], call the service, and render. Rate limits apply per-IP to
//! register/login/refresh; `/me` routes require a valid JWT.

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use super::models::{
    AcceptInvitationRequest, InvitationTokenRequest, LoginRequest, LogoutRequest, RefreshRequest,
    RegisterRequest, UpdateMeRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{rate_limit, require_auth, RateLimit};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the auth router (mounted at `/auth`).
pub fn routes(state: AppState) -> Router {
    let public = Router::new()
        .merge(rate_limited(
            "/register",
            post(register),
            state.config.rate_limit_register_per_minute,
            state.config.trust_proxy_headers,
        ))
        .merge(rate_limited(
            "/login",
            post(login),
            state.config.rate_limit_login_per_minute,
            state.config.trust_proxy_headers,
        ))
        .merge(rate_limited(
            "/refresh",
            post(refresh),
            state.config.rate_limit_refresh_per_minute,
            state.config.trust_proxy_headers,
        ))
        // Invitation links are public; they share the registration limit.
        .merge(rate_limited(
            "/invitation",
            post(preview_invitation),
            state.config.rate_limit_register_per_minute,
            state.config.trust_proxy_headers,
        ))
        .merge(rate_limited(
            "/accept-invitation",
            post(accept_invitation),
            state.config.rate_limit_register_per_minute,
            state.config.trust_proxy_headers,
        ))
        .route("/logout", post(logout));

    let protected = Router::new()
        .route("/me", get(get_me).patch(patch_me).delete(delete_me))
        .route_layer(from_fn_with_state(state.clone(), require_auth));

    public.merge(protected).with_state(state)
}

/// Wraps a single route with a per-IP rate-limit layer.
fn rate_limited(
    path: &str,
    handler: axum::routing::MethodRouter<AppState>,
    limit: u32,
    trust_proxy_headers: bool,
) -> Router<AppState> {
    let layer = from_fn_with_state(
        RateLimit::per_minute(limit, trust_proxy_headers),
        rate_limit,
    );
    Router::new().route(path, handler).route_layer(layer)
}

async fn register(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RegisterRequest>,
) -> Result<Response, ApiError> {
    let resp = service::register(&state, req).await?;
    let location = format!("/api/v1/users/{}", resp.user_id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(resp),
    )
        .into_response())
}

async fn preview_invitation(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<InvitationTokenRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::preview_invitation(&state, &req.token).await?).into_response())
}

async fn accept_invitation(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<AcceptInvitationRequest>,
) -> Result<Response, ApiError> {
    let resp = service::accept_invitation(&state, req).await?;
    Ok((StatusCode::CREATED, Json(resp)).into_response())
}

async fn login(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<LoginRequest>,
) -> Result<Response, ApiError> {
    let resp = service::login(&state, req).await?;
    Ok(Json(resp).into_response())
}

async fn refresh(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RefreshRequest>,
) -> Result<Response, ApiError> {
    let resp = service::refresh(&state, &req.refresh_token).await?;
    Ok(Json(resp).into_response())
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<LogoutRequest>,
) -> Result<Response, ApiError> {
    // The access token, when sent, is revoked too. A missing or invalid one is
    // ignored: logout must work with an expired token.
    let access = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| state.jwt.verify(token).ok());
    service::logout(&state, &req.refresh_token, access).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn get_me(State(state): State<AppState>, ctx: RequestContext) -> Result<Response, ApiError> {
    let me = service::me(&state, ctx.user_id()?).await?;
    Ok(Json(me).into_response())
}

async fn patch_me(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<UpdateMeRequest>,
) -> Result<Response, ApiError> {
    let me = service::update_me(&state, ctx.user_id()?, req).await?;
    Ok(Json(me).into_response())
}

async fn delete_me(
    State(state): State<AppState>,
    ctx: RequestContext,
) -> Result<Response, ApiError> {
    service::delete_me(&state, ctx.user_id()?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
