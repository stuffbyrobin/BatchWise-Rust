//! Label-design HTTP handlers and router.
//!
//! Port of the Go labeldesign handlers (three chi routers → one axum `routes()`).
//! All routes require auth and the `label_design` feature flag. Brand-asset
//! upload is multipart (the documented exception to the JSON-only rule); asset
//! fetch and `render.pdf` return binary bodies.

use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CreateBrandProfileRequest, CreateLabelDesignRequest, ListFilter, PatchBrandProfileRequest,
    PatchLabelDesignRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the label-design routers (brand-assets, brand-profiles, label-designs),
/// gated by auth + the `label_design` feature flag.
pub fn routes(state: AppState) -> Router {
    let assets = Router::new()
        .route("/", post(upload_asset))
        .route("/{id}", get(get_asset).delete(delete_asset));
    let profiles = Router::new()
        .route("/", post(create_profile).get(list_profiles))
        .route(
            "/{id}",
            get(get_profile).patch(patch_profile).delete(delete_profile),
        );
    let designs = Router::new()
        .route("/", post(create_design).get(list_designs))
        .route(
            "/{id}",
            get(get_design).patch(patch_design).delete(delete_design),
        )
        .route("/{id}/render", get(render_model))
        .route("/{id}/render.pdf", get(render_pdf_handler));

    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "label_design", req, next).await }
    });

    Router::new()
        .nest("/brand-assets", assets)
        .nest("/brand-profiles", profiles)
        .nest("/label-designs", designs)
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct DesignQuery {
    kind: Option<String>,
    batch_id: Option<String>,
    recipe_id: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

// ---- brand assets ----

async fn upload_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    let tenant_id = ctx.tenant_id()?;

    let mut found: Option<(String, String, Vec<u8>)> = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => return Err(ApiError::validation("file", "invalid multipart form")),
        };
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("").to_string();
        let content_type = field.content_type().unwrap_or("").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|_| ApiError::validation("file", "invalid multipart form"))?
            .to_vec();
        found = Some((filename, content_type, data));
        break;
    }

    let (filename, content_type, data) =
        found.ok_or_else(|| ApiError::validation("file", "missing 'file' part"))?;

    let a = service::upload_asset(&state, tenant_id, &filename, &content_type, &data).await?;
    let location = format!("/api/v1/brand-assets/{}", a.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(a)).into_response())
}

async fn get_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let (asset, data) = service::get_asset_data(&state, ctx.tenant_id()?, id).await?;
    let len = data.len();
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, asset.content_type),
            (header::CONTENT_LENGTH, len.to_string()),
        ],
        Body::from(data),
    )
        .into_response())
}

async fn delete_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_asset(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- brand profiles ----

async fn create_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateBrandProfileRequest>,
) -> Result<Response, ApiError> {
    let p = service::create_profile(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/brand-profiles/{}", p.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(p)).into_response())
}

async fn list_profiles(
    State(state): State<AppState>,
    ctx: RequestContext,
) -> Result<Response, ApiError> {
    let items = service::list_profiles(&state, ctx.tenant_id()?).await?;
    Ok(Json(serde_json::json!({ "items": items })).into_response())
}

async fn get_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_profile(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchBrandProfileRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_profile(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_profile(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_profile(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- designs ----

async fn create_design(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateLabelDesignRequest>,
) -> Result<Response, ApiError> {
    let d = service::create_design(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/label-designs/{}", d.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(d)).into_response())
}

async fn list_designs(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<DesignQuery>,
) -> Result<Response, ApiError> {
    // Unparseable batch_id/recipe_id are silently ignored (matches the Go handler).
    let batch_id = q
        .batch_id
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(&s).ok());
    let recipe_id = q
        .recipe_id
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(&s).ok());
    let filter = ListFilter {
        kind: q.kind.filter(|s| !s.is_empty()),
        batch_id,
        recipe_id,
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
        sort: q.sort.unwrap_or_default(),
    };
    Ok(Json(service::list_designs(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_design(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_design(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_design(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchLabelDesignRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_design(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_design(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_design(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn render_model(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::render(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn render_pdf_handler(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let tenant_id = ctx.tenant_id()?;
    // Load the design first for its name (404 if missing), then render.
    let d = service::get_design(&state, tenant_id, id).await?;
    let pdf = service::render_pdf(&state, tenant_id, id).await?;
    let len = pdf.len();
    let disposition = format!(
        "inline; filename=\"{}.pdf\"",
        service::sanitize_filename(&d.name)
    );
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CONTENT_LENGTH, len.to_string()),
        ],
        Body::from(pdf),
    )
        .into_response())
}
