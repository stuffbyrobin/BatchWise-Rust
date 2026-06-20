//! Fermenter HTTP handlers and router (mounted at `/api/v1/fermenters`).

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{CreateRequest, ListFilter, UpdateRequest};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the fermenter router (mounted at `/fermenters`).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).patch(patch).delete(delete))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    name: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn create(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateRequest>,
) -> Result<Response, ApiError> {
    let fermenter = service::create(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/fermenters/{}", fermenter.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(fermenter),
    )
        .into_response())
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let filter = ListFilter {
        name: q.name,
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
    let fermenter = service::get(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(fermenter).into_response())
}

async fn patch(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateRequest>,
) -> Result<Response, ApiError> {
    let fermenter = service::update(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(fermenter).into_response())
}

async fn delete(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
