//! Yeast kinetics HTTP handlers and router.
//!
//! Port of the Go `internal/yeastkinetics/handler.go`. All routes require auth
//! and are mounted at `/api/v1/yeast-kinetics`.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{CreateRequest, ListFilter, PatchRequest};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the yeast kinetics router (mounted at `/yeast-kinetics`).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create).get(list))
        .route(
            "/{id}",
            get(get_one).put(replace).patch(patch).delete(delete),
        )
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    yeast_id: Option<Uuid>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn create(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateRequest>,
) -> Result<Response, ApiError> {
    let k = service::create(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/yeast-kinetics/{}", k.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(k)).into_response())
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let filter = ListFilter {
        yeast_id: q.yeast_id,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
        sort: q.sort.unwrap_or_default(),
    };
    let page = service::list(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn get_one(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let k = service::get(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(k).into_response())
}

async fn replace(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateRequest>,
) -> Result<Response, ApiError> {
    let k = service::replace(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(k).into_response())
}

async fn patch(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchRequest>,
) -> Result<Response, ApiError> {
    let k = service::patch(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(k).into_response())
}

async fn delete(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
