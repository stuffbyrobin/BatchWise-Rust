//! Yeast banking HTTP handlers and router (`/yeast-bank`).
//!
//! Port of the Go yeast banking handlers. All routes require auth and the
//! `yeast_banking` feature flag (tier gate).

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CreatePropagationRequest, CreateYeastBankRequest, HarvestRequest, PatchPropagationRequest,
    PatchYeastBankRequest, YeastBankFilter,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the `/yeast-bank` router, gated by auth + the `yeast_banking`
/// feature flag.
pub fn routes(state: AppState) -> Router {
    let entries = Router::new()
        .route("/", get(list_entries).post(create_entry))
        .route(
            "/{id}",
            get(get_entry).patch(patch_entry).delete(delete_entry),
        )
        .route("/{id}/harvest", post(harvest))
        .route(
            "/{id}/propagations",
            get(list_propagations).post(create_propagation),
        )
        .route(
            "/{id}/propagations/{prop_id}",
            axum::routing::patch(patch_propagation).delete(delete_propagation),
        );

    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "yeast_banking", req, next).await }
    });

    Router::new()
        .nest("/yeast-bank", entries)
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct EntryQuery {
    status: Option<String>,
    library_yeast_id: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PageQuery {
    page: Option<i64>,
    page_size: Option<i64>,
}

// ---- entries ----

async fn list_entries(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<EntryQuery>,
) -> Result<Response, ApiError> {
    // An unparseable library_yeast_id is silently ignored (matches the Go handler).
    let library_yeast_id = q
        .library_yeast_id
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(&s).ok());
    let status = q.status.filter(|s| !s.is_empty());
    let filter = YeastBankFilter {
        status,
        library_yeast_id,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
    };
    Ok(Json(service::list_entries(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn create_entry(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateYeastBankRequest>,
) -> Result<Response, ApiError> {
    let e = service::create_entry(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/yeast-bank/{}", e.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(e)).into_response())
}

async fn get_entry(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_entry(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_entry(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchYeastBankRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_entry(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_entry(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_entry(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn harvest(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<HarvestRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::harvest(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

// ---- propagations ----

async fn list_propagations(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    Query(q): Query<PageQuery>,
) -> Result<Response, ApiError> {
    let page = q.page.unwrap_or(0);
    let page_size = q.page_size.unwrap_or(0);
    Ok(
        Json(service::list_propagations(&state, ctx.tenant_id()?, id, page, page_size).await?)
            .into_response(),
    )
}

async fn create_propagation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreatePropagationRequest>,
) -> Result<Response, ApiError> {
    let p = service::create_propagation(&state, ctx.tenant_id()?, id, req).await?;
    let location = format!("/api/v1/yeast-bank/{}/propagations/{}", id, p.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(p)).into_response())
}

async fn patch_propagation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((id, prop_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(req): ValidatedJson<PatchPropagationRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::patch_propagation(&state, ctx.tenant_id()?, id, prop_id, req).await?)
            .into_response(),
    )
}

async fn delete_propagation(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((id, prop_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    service::delete_propagation(&state, ctx.tenant_id()?, id, prop_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
