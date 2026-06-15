//! Library HTTP handlers and router.
//!
//! Port of the Go `internal/library/handler.go`. Every route requires a valid
//! JWT (the `require_auth` layer populates tenant/user ids on the request
//! context). Handlers decode + validate via [`ValidatedJson`], call the service,
//! and render. The router is mounted at `/api/v1/library` by the orchestrator.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use super::models::{
    EquipmentFilter, EquipmentRequest, FermentableFilter, FermentableRequest, MashFilter,
    MashProfileRequest, PatchEquipmentRequest, PatchFermentableRequest, PatchMashProfileRequest,
    PatchStyleRequest, PatchYeastRequest, StyleFilter, StyleRequest, YeastFilter, YeastRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the library router (mounted at `/api/v1/library`).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/styles", get(list_styles).post(create_style))
        .route(
            "/styles/{id}",
            get(get_style)
                .put(replace_style)
                .patch(patch_style)
                .delete(delete_style),
        )
        .route(
            "/equipment-profiles",
            get(list_equipment).post(create_equipment),
        )
        .route(
            "/equipment-profiles/{id}",
            get(get_equipment)
                .put(replace_equipment)
                .patch(patch_equipment)
                .delete(delete_equipment),
        )
        .route(
            "/mash-profiles",
            get(list_mash_profiles).post(create_mash_profile),
        )
        .route(
            "/mash-profiles/{id}",
            get(get_mash_profile)
                .put(replace_mash_profile)
                .patch(patch_mash_profile)
                .delete(delete_mash_profile),
        )
        .route("/yeasts", get(list_yeasts).post(create_yeast))
        .route(
            "/yeasts/{id}",
            get(get_yeast)
                .put(replace_yeast)
                .patch(patch_yeast)
                .delete(delete_yeast),
        )
        .route(
            "/fermentables",
            get(list_fermentables).post(create_fermentable),
        )
        .route(
            "/fermentables/{id}",
            get(get_fermentable)
                .put(replace_fermentable)
                .patch(patch_fermentable)
                .delete(delete_fermentable),
        )
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

/// Parses a path UUID, returning a field-level validation error (matching Go).
fn parse_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::validation("id", "must be a valid UUID"))
}

// ---- Style handlers ----

async fn list_styles(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(filter): Query<StyleFilter>,
) -> Result<Response, ApiError> {
    let result = service::list_styles(&state.pool, ctx.tenant_id()?, filter).await?;
    Ok(Json(result).into_response())
}

async fn get_style(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let s = service::get_style(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(Json(s).into_response())
}

async fn create_style(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<StyleRequest>,
) -> Result<Response, ApiError> {
    let s = service::create_style(&state.pool, ctx.tenant_id()?, req).await?;
    Ok((StatusCode::CREATED, Json(s)).into_response())
}

async fn replace_style(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<StyleRequest>,
) -> Result<Response, ApiError> {
    let s = service::replace_style(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(s).into_response())
}

async fn patch_style(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<PatchStyleRequest>,
) -> Result<Response, ApiError> {
    let s = service::patch_style(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(s).into_response())
}

async fn delete_style(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    service::delete_style(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- Equipment handlers ----

async fn list_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(filter): Query<EquipmentFilter>,
) -> Result<Response, ApiError> {
    let result = service::list_equipment(&state.pool, ctx.tenant_id()?, filter).await?;
    Ok(Json(result).into_response())
}

async fn get_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let ep = service::get_equipment(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(Json(ep).into_response())
}

async fn create_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<EquipmentRequest>,
) -> Result<Response, ApiError> {
    let ep = service::create_equipment(&state.pool, ctx.tenant_id()?, req).await?;
    Ok((StatusCode::CREATED, Json(ep)).into_response())
}

async fn replace_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<EquipmentRequest>,
) -> Result<Response, ApiError> {
    let ep = service::replace_equipment(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(ep).into_response())
}

async fn patch_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<PatchEquipmentRequest>,
) -> Result<Response, ApiError> {
    let ep = service::patch_equipment(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(ep).into_response())
}

async fn delete_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    service::delete_equipment(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- Mash profile handlers ----

async fn list_mash_profiles(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(filter): Query<MashFilter>,
) -> Result<Response, ApiError> {
    let result = service::list_mash_profiles(&state.pool, ctx.tenant_id()?, filter).await?;
    Ok(Json(result).into_response())
}

async fn get_mash_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let mp = service::get_mash_profile(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(Json(mp).into_response())
}

async fn create_mash_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<MashProfileRequest>,
) -> Result<Response, ApiError> {
    let mp = service::create_mash_profile(&state.pool, ctx.tenant_id()?, req).await?;
    Ok((StatusCode::CREATED, Json(mp)).into_response())
}

async fn replace_mash_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<MashProfileRequest>,
) -> Result<Response, ApiError> {
    let mp =
        service::replace_mash_profile(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(mp).into_response())
}

async fn patch_mash_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<PatchMashProfileRequest>,
) -> Result<Response, ApiError> {
    let mp =
        service::patch_mash_profile(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(mp).into_response())
}

async fn delete_mash_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    service::delete_mash_profile(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- Yeast handlers ----

async fn list_yeasts(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(filter): Query<YeastFilter>,
) -> Result<Response, ApiError> {
    let result = service::list_yeasts(&state.pool, ctx.tenant_id()?, filter).await?;
    Ok(Json(result).into_response())
}

async fn get_yeast(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let y = service::get_yeast(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(Json(y).into_response())
}

async fn create_yeast(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<YeastRequest>,
) -> Result<Response, ApiError> {
    let y = service::create_yeast(&state.pool, ctx.tenant_id()?, req).await?;
    Ok((StatusCode::CREATED, Json(y)).into_response())
}

async fn replace_yeast(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<YeastRequest>,
) -> Result<Response, ApiError> {
    let y = service::replace_yeast(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(y).into_response())
}

async fn patch_yeast(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<PatchYeastRequest>,
) -> Result<Response, ApiError> {
    let y = service::patch_yeast(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(y).into_response())
}

async fn delete_yeast(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    service::delete_yeast(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- Library fermentable handlers ----

async fn list_fermentables(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(filter): Query<FermentableFilter>,
) -> Result<Response, ApiError> {
    let result = service::list_fermentables(&state.pool, ctx.tenant_id()?, filter).await?;
    Ok(Json(result).into_response())
}

async fn get_fermentable(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let f = service::get_fermentable(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(Json(f).into_response())
}

async fn create_fermentable(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<FermentableRequest>,
) -> Result<Response, ApiError> {
    let f = service::create_fermentable(&state.pool, ctx.tenant_id()?, req).await?;
    Ok((StatusCode::CREATED, Json(f)).into_response())
}

async fn replace_fermentable(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<FermentableRequest>,
) -> Result<Response, ApiError> {
    let f =
        service::replace_fermentable(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(f).into_response())
}

async fn patch_fermentable(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<PatchFermentableRequest>,
) -> Result<Response, ApiError> {
    let f = service::patch_fermentable(&state.pool, ctx.tenant_id()?, parse_id(&id)?, req).await?;
    Ok(Json(f).into_response())
}

async fn delete_fermentable(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    service::delete_fermentable(&state.pool, ctx.tenant_id()?, parse_id(&id)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
