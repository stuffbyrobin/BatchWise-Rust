//! Inventory HTTP handlers and router.
//!
//! Port of the Go `internal/inventory/handler.go`. All routes require auth.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CreateRequest, DeductRequest, ListFilter, MovementFilter, PatchRequest, StockInRequest,
    SummaryFilter,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the inventory router (mounted at `/inventory`).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create).get(list))
        .route("/deduct", post(deduct))
        .route("/summary", get(summary))
        .route("/stock-movements", get(list_movements))
        .route(
            "/{id}",
            get(get_one).put(replace).patch(patch).delete(delete),
        )
        .route("/{id}/stock", post(append_stock))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    r#type: Option<String>,
    name: Option<String>,
    lot_number: Option<String>,
    expiring_before: Option<String>,
    expiring_within_days: Option<i32>,
    out_of_stock: Option<bool>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SummaryQuery {
    r#type: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MovementQuery {
    ingredient_id: Option<Uuid>,
    reference_type: Option<String>,
    reference_id: Option<Uuid>,
    from_date: Option<String>,
    to_date: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn create(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateRequest>,
) -> Result<Response, ApiError> {
    let ing = service::create(&state, ctx.tenant_id()?, ctx.user_id()?, req).await?;
    let location = format!("/api/v1/inventory/{}", ing.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(ing),
    )
        .into_response())
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let filter = ListFilter {
        r#type: q.r#type,
        name: q.name,
        lot_number: q.lot_number,
        expiring_before: q.expiring_before,
        expiring_within_days: q.expiring_within_days,
        out_of_stock: q.out_of_stock.unwrap_or(false),
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
    let ing = service::get(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(ing).into_response())
}

async fn replace(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateRequest>,
) -> Result<Response, ApiError> {
    let ing = service::replace(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(ing).into_response())
}

async fn patch(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchRequest>,
) -> Result<Response, ApiError> {
    let ing = service::patch(&state, ctx.tenant_id()?, id, req).await?;
    Ok(Json(ing).into_response())
}

async fn delete(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn append_stock(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<StockInRequest>,
) -> Result<Response, ApiError> {
    let ing = service::append_stock(&state, ctx.tenant_id()?, ctx.user_id()?, id, req).await?;
    let location = format!("/api/v1/inventory/{id}");
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(ing),
    )
        .into_response())
}

async fn deduct(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<DeductRequest>,
) -> Result<Response, ApiError> {
    let result = service::deduct(&state, ctx.tenant_id()?, ctx.user_id()?, req).await?;
    Ok(Json(result).into_response())
}

async fn summary(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<SummaryQuery>,
) -> Result<Response, ApiError> {
    let filter = SummaryFilter {
        r#type: q.r#type,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::summary(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn list_movements(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<MovementQuery>,
) -> Result<Response, ApiError> {
    let filter = MovementFilter {
        ingredient_id: q.ingredient_id,
        reference_type: q.reference_type,
        reference_id: q.reference_id,
        from_date: q.from_date,
        to_date: q.to_date,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::list_movements(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}
