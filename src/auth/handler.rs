//! Auth HTTP handlers and router.
//!
//! Port of the Go `internal/auth/handler.go`. Handlers decode + validate via
//! [`ValidatedJson`], call the service, and render. Rate limits apply per-IP to
//! register/login/refresh; `/me` routes require a valid JWT.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{from_fn, from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use super::models::{
    LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest, UpdateMeRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{client_ip, require_auth, RateLimiter};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the auth router (mounted at `/auth`).
pub fn routes(state: AppState) -> Router {
    let public = Router::new()
        .merge(rate_limited(
            "/register",
            post(register),
            state.config.rate_limit_register_per_minute,
        ))
        .merge(rate_limited(
            "/login",
            post(login),
            state.config.rate_limit_login_per_minute,
        ))
        .merge(rate_limited(
            "/refresh",
            post(refresh),
            state.config.rate_limit_refresh_per_minute,
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
) -> Router<AppState> {
    let limiter = Arc::new(RateLimiter::per_minute(limit));
    let layer = from_fn(move |req: Request, next: Next| {
        let limiter = limiter.clone();
        async move {
            match limiter.check(&client_ip(&req)) {
                Ok(()) => Ok(next.run(req).await),
                Err(retry) => Err(ApiError::rate_limited(retry)),
            }
        }
    });
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
    ValidatedJson(req): ValidatedJson<LogoutRequest>,
) -> Result<Response, ApiError> {
    service::logout(&state, &req.refresh_token).await?;
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
