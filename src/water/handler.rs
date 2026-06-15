//! Water chemistry HTTP handlers and routers.
//!
//! Port of the Go water handlers (profiles, adjustments, calculate). All routes
//! require auth; there is no feature gate (water is available on every tier).

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    AdjustmentFilter, CalculateRequest, CreateWaterAdjustmentRequest, CreateWaterProfileRequest,
    PatchWaterAdjustmentRequest, PatchWaterProfileRequest, ProfileFilter,
    UpdateWaterAdjustmentRequest, UpdateWaterProfileRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the water routers (water-profiles, water-adjustments), gated by auth.
pub fn routes(state: AppState) -> Router {
    let profiles = Router::new()
        .route("/", get(list_profiles).post(create_profile))
        .route(
            "/{id}",
            get(get_profile)
                .put(replace_profile)
                .patch(patch_profile)
                .delete(delete_profile),
        );
    let adjustments = Router::new()
        .route("/", get(list_adjustments).post(create_adjustment))
        // `/calculate` must be registered before `/{id}`.
        .route("/calculate", post(calculate))
        .route(
            "/{id}",
            get(get_adjustment)
                .put(replace_adjustment)
                .patch(patch_adjustment)
                .delete(delete_adjustment),
        );

    Router::new()
        .nest("/water-profiles", profiles)
        .nest("/water-adjustments", adjustments)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ProfileQuery {
    sort: Option<String>,
    page: Option<i32>,
    page_size: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct AdjustmentQuery {
    batch_id: Option<Uuid>,
    recipe_id: Option<Uuid>,
    sort: Option<String>,
    page: Option<i32>,
    page_size: Option<i32>,
}

// ---- Profiles ----

async fn create_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateWaterProfileRequest>,
) -> Result<Response, ApiError> {
    let p = service::create_water_profile(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/water-profiles/{}", p.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(p)).into_response())
}

async fn list_profiles(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ProfileQuery>,
) -> Result<Response, ApiError> {
    let filter = ProfileFilter {
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    Ok(Json(service::list_water_profiles(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_water_profile(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn replace_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateWaterProfileRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::update_water_profile(&state, ctx.tenant_id()?, id, req).await?)
            .into_response(),
    )
}

async fn patch_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchWaterProfileRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::patch_water_profile(&state, ctx.tenant_id()?, id, req).await?)
            .into_response(),
    )
}

async fn delete_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_water_profile(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- Adjustments ----

async fn create_adjustment(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateWaterAdjustmentRequest>,
) -> Result<Response, ApiError> {
    let a = service::create_water_adjustment(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/water-adjustments/{}", a.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(a)).into_response())
}

async fn list_adjustments(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<AdjustmentQuery>,
) -> Result<Response, ApiError> {
    let filter = AdjustmentFilter {
        batch_id: q.batch_id,
        recipe_id: q.recipe_id,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    Ok(
        Json(service::list_water_adjustments(&state, ctx.tenant_id()?, filter).await?)
            .into_response(),
    )
}

async fn calculate(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CalculateRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::calculate(&state, ctx.tenant_id()?, req).await?).into_response())
}

async fn get_adjustment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_water_adjustment(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn replace_adjustment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateWaterAdjustmentRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::update_water_adjustment(&state, ctx.tenant_id()?, id, req).await?)
            .into_response(),
    )
}

async fn patch_adjustment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchWaterAdjustmentRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::patch_water_adjustment(&state, ctx.tenant_id()?, id, req).await?)
            .into_response(),
    )
}

async fn delete_adjustment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_water_adjustment(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
