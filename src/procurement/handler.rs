//! Procurement HTTP handlers and routers (suppliers, purchase-orders).
//!
//! Port of the Go procurement handlers. All routes require auth and the
//! `procurement` feature flag (tier gate).

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CreateLineRequest, CreatePORequest, CreateSupplierRequest, POFilter, PatchLineRequest,
    PatchPORequest, PatchSupplierRequest, ReceiveRequest, SupplierFilter,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the procurement routers (suppliers, purchase-orders), gated by auth +
/// the `procurement` feature flag.
pub fn routes(state: AppState) -> Router {
    let suppliers = Router::new()
        .route("/", get(list_suppliers).post(create_supplier))
        .route(
            "/{id}",
            get(get_supplier)
                .patch(patch_supplier)
                .delete(delete_supplier),
        );
    let pos = Router::new()
        .route("/", get(list_pos).post(create_po))
        .route("/{id}", get(get_po).patch(patch_po).delete(delete_po))
        .route("/{id}/lines", post(add_line))
        .route(
            "/{id}/lines/{line_id}",
            patch(patch_line).delete(delete_line),
        )
        .route("/{id}/receive", post(receive_po));

    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "procurement", req, next).await }
    });

    Router::new()
        .nest("/suppliers", suppliers)
        .nest("/purchase-orders", pos)
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct SupplierQuery {
    search: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct POQuery {
    supplier_id: Option<String>,
    status: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

// ---- suppliers ----

async fn list_suppliers(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<SupplierQuery>,
) -> Result<Response, ApiError> {
    let filter = SupplierFilter {
        search: q.search.unwrap_or_default(),
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
    };
    Ok(Json(service::list_suppliers(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn create_supplier(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateSupplierRequest>,
) -> Result<Response, ApiError> {
    let s = service::create_supplier(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/suppliers/{}", s.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(s)).into_response())
}

async fn get_supplier(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_supplier(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_supplier(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchSupplierRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_supplier(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_supplier(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_supplier(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- purchase orders ----

async fn list_pos(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<POQuery>,
) -> Result<Response, ApiError> {
    // An unparseable supplier_id is silently ignored (matches the Go handler).
    let supplier_id = q
        .supplier_id
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(&s).ok());
    let status = q.status.filter(|s| !s.is_empty());
    let filter = POFilter {
        supplier_id,
        status,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
    };
    Ok(Json(service::list_pos(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn create_po(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreatePORequest>,
) -> Result<Response, ApiError> {
    let po = service::create_po(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/purchase-orders/{}", po.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(po),
    )
        .into_response())
}

async fn get_po(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_po(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_po(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchPORequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_po(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_po(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_po(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn add_line(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateLineRequest>,
) -> Result<Response, ApiError> {
    let line = service::add_line(&state, ctx.tenant_id()?, id, req).await?;
    let location = format!("/api/v1/purchase-orders/{}/lines/{}", id, line.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(line),
    )
        .into_response())
}

async fn patch_line(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((id, line_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(req): ValidatedJson<PatchLineRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::patch_line(&state, ctx.tenant_id()?, id, line_id, req).await?)
            .into_response(),
    )
}

async fn delete_line(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((id, line_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    service::delete_line(&state, ctx.tenant_id()?, id, line_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn receive_po(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<ReceiveRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::receive_po(&state, ctx.tenant_id()?, id, req).await?).into_response())
}
