//! Batch HTTP handlers and router.
//!
//! Port of the Go `internal/batch/handler.go`. All routes require auth.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CreateRequest, ListFilter, PatchIngredientsRequest, TransitionRequest, UpdateRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the batch router (mounted at `/batches`).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route(
            "/{id}",
            get(get_one).put(update).patch(update).delete(delete),
        )
        .route("/{id}/transition", post(transition))
        .route("/{id}/ingredients", patch(patch_ingredients))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    status: Option<String>,
    recipe_id: Option<Uuid>,
    brew_date_from: Option<String>,
    brew_date_to: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn create(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateRequest>,
) -> Result<Response, ApiError> {
    let result = service::create(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/batches/{}", result.batch.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(result),
    )
        .into_response())
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let filter = ListFilter {
        status: q.status,
        recipe_id: q.recipe_id,
        brew_date_from: q.brew_date_from,
        brew_date_to: q.brew_date_to,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::list(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn get_one(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let batch = service::get(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(batch).into_response())
}

async fn update(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateRequest>,
) -> Result<Response, ApiError> {
    let batch = service::update(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(batch).into_response())
}

async fn delete(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn transition(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<TransitionRequest>,
) -> Result<Response, ApiError> {
    let batch =
        service::transition(&state, ctx.tenant_id()?, ctx.user_id()?, id, &req.to_status).await?;
    Ok(Json(batch).into_response())
}

async fn patch_ingredients(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchIngredientsRequest>,
) -> Result<Response, ApiError> {
    let batch = service::patch_ingredients(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(batch).into_response())
}
