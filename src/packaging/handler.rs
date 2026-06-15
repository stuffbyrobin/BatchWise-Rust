//! Packaging HTTP handlers and routers (packaging-runs, distribution-movements).
//!
//! Port of the Go packaging handlers. All routes require auth and the
//! `packaging` feature flag (tier gate).

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CreateMovementRequest, CreatePackagingRunRequest, ListMovementsFilter, ListPackagingRunsFilter,
    PatchPackagingRunRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the packaging routers (packaging-runs, distribution-movements),
/// gated by auth + the `packaging` feature flag.
pub fn routes(state: AppState) -> Router {
    let runs = Router::new()
        .route("/", get(list_runs).post(create_run))
        .route("/{id}", get(get_run).patch(patch_run).delete(delete_run));
    let movements = Router::new()
        .route("/", get(list_movements).post(create_movement))
        .route("/{id}", get(get_movement).delete(delete_movement));

    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "packaging", req, next).await }
    });

    Router::new()
        .nest("/packaging-runs", runs)
        .nest("/distribution-movements", movements)
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct RunQuery {
    batch_id: Option<Uuid>,
    format: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MovementQuery {
    packaging_run_id: Option<Uuid>,
    order_id: Option<Uuid>,
    movement_type: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

// ---- packaging runs ----

async fn create_run(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreatePackagingRunRequest>,
) -> Result<Response, ApiError> {
    let run = service::create_run(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/packaging-runs/{}", run.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(run),
    )
        .into_response())
}

async fn list_runs(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<RunQuery>,
) -> Result<Response, ApiError> {
    let filter = ListPackagingRunsFilter {
        batch_id: q.batch_id,
        format: q.format,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    Ok(Json(service::list_runs(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_run(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_run(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_run(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchPackagingRunRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_run(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_run(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_run(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- distribution movements ----

async fn create_movement(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateMovementRequest>,
) -> Result<Response, ApiError> {
    let m = service::create_movement(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/distribution-movements/{}", m.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(m)).into_response())
}

async fn list_movements(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<MovementQuery>,
) -> Result<Response, ApiError> {
    let filter = ListMovementsFilter {
        packaging_run_id: q.packaging_run_id,
        order_id: q.order_id,
        movement_type: q.movement_type,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    Ok(Json(service::list_movements(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_movement(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_movement(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn delete_movement(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_movement(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
