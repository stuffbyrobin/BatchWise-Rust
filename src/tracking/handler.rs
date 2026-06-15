//! Container-tracking HTTP handlers and routers.
//!
//! Port of the Go tracking handlers (assets, logs, QR). All routes require auth
//! and the `tracking` feature flag (tier gate).

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    AssetFilter, CreateAssetRequest, DeliverRequest, FillRequest, LogFilter, PatchAssetRequest,
    ReturnRequest, SetStatusRequest, UpdateAssetRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the tracking routers (container-assets, container-logs, qr-codes),
/// gated by auth + the `tracking` feature flag.
pub fn routes(state: AppState) -> Router {
    let assets = Router::new()
        .route("/", get(list_assets).post(create_asset))
        .route(
            "/{id}",
            get(get_asset)
                .put(update_asset)
                .patch(patch_asset)
                .delete(delete_asset),
        )
        .route("/{id}/fill", post(fill))
        .route("/{id}/deliver", post(deliver))
        .route("/{id}/return", post(return_asset))
        .route("/{id}/status", post(set_status));
    let logs = Router::new()
        .route("/", get(list_logs))
        .route("/{id}", get(get_log));
    let qr = Router::new()
        .route("/{container_id}/a", get(qr_a))
        .route("/{container_id}/b", get(qr_b));

    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "tracking", req, next).await }
    });

    Router::new()
        .nest("/container-assets", assets)
        .nest("/container-logs", logs)
        .nest("/qr-codes", qr)
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct AssetQuery {
    status: Option<String>,
    container_type: Option<String>,
    current_batch_id: Option<Uuid>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    container_id: Option<Uuid>,
    event_type: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn create_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateAssetRequest>,
) -> Result<Response, ApiError> {
    let a = service::create_asset(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/container-assets/{}", a.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(a)).into_response())
}

async fn list_assets(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<AssetQuery>,
) -> Result<Response, ApiError> {
    let filter = AssetFilter {
        status: q.status,
        container_type: q.container_type,
        current_batch_id: q.current_batch_id,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    Ok(Json(service::list_assets(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_asset(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn update_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateAssetRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::update_asset(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn patch_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchAssetRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_asset(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_asset(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn fill(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<FillRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::fill(&state, ctx.tenant_id()?, ctx.user_id()?, id, req).await?)
            .into_response(),
    )
}

async fn deliver(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<DeliverRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::deliver(&state, ctx.tenant_id()?, ctx.user_id()?, id, req).await?)
            .into_response(),
    )
}

async fn return_asset(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<ReturnRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::return_asset(&state, ctx.tenant_id()?, ctx.user_id()?, id, req).await?)
            .into_response(),
    )
}

async fn set_status(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<SetStatusRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::set_status(&state, ctx.tenant_id()?, ctx.user_id()?, id, req).await?)
            .into_response(),
    )
}

async fn list_logs(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<LogQuery>,
) -> Result<Response, ApiError> {
    let filter = LogFilter {
        container_id: q.container_id,
        event_type: q.event_type,
        from_date: q.from_date,
        to_date: q.to_date,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    Ok(Json(service::list_logs(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_log(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_log(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn qr_a(
    state: State<AppState>,
    ctx: RequestContext,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    qr_response(state, ctx, headers, id, "a").await
}

async fn qr_b(
    state: State<AppState>,
    ctx: RequestContext,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    qr_response(state, ctx, headers, id, "b").await
}

/// Returns the QR as JSON when `Accept: application/json`, else raw PNG bytes.
async fn qr_response(
    State(state): State<AppState>,
    ctx: RequestContext,
    headers: HeaderMap,
    id: Uuid,
    variant: &str,
) -> Result<Response, ApiError> {
    let result = service::generate_qr(&state, ctx.tenant_id()?, id, variant).await?;
    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|a| a.contains("application/json"));
    if wants_json {
        return Ok(Json(result).into_response());
    }
    let png = base64::engine::general_purpose::STANDARD
        .decode(result.png_base64.as_bytes())
        .map_err(|e| ApiError::internal(format!("qr decode: {e}")))?;
    Ok(([(header::CONTENT_TYPE, "image/png")], Body::from(png)).into_response())
}
