//! Fermentation HTTP handlers and router.
//!
//! Port of the Go `internal/fermentation/handler.go`. The Go module mounts at
//! `/batches/{id}/fermentation`; here the router uses relative paths and is
//! merged into the `/batches` nest by the orchestrator. All routes require auth
//! and the `fermentation` feature flag.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{CreateReadingRequest, PatchReadingRequest, ReadingFilter};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the fermentation router (paths relative to `/batches`), gated by auth +
/// the `fermentation` feature flag. Merged into the `/batches` nest so the full
/// paths are `/batches/{id}/fermentation[/{reading_id}]`.
pub fn routes(state: AppState) -> Router {
    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "fermentation", req, next).await }
    });
    Router::new()
        .route("/{id}/fermentation", get(list).post(create))
        .route(
            "/{id}/fermentation/{reading_id}",
            axum::routing::patch(patch).delete(delete),
        )
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    stage: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(batch_id): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let filter = ReadingFilter {
        stage: q.stage,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    Ok(
        Json(service::list_readings(&state, ctx.tenant_id()?, batch_id, filter).await?)
            .into_response(),
    )
}

async fn create(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(batch_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateReadingRequest>,
) -> Result<Response, ApiError> {
    let rd = service::create_reading(&state, ctx.tenant_id()?, batch_id, req).await?;
    let location = format!("/api/v1/batches/{batch_id}/fermentation/{}", rd.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(rd),
    )
        .into_response())
}

async fn patch(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((batch_id, reading_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(req): ValidatedJson<PatchReadingRequest>,
) -> Result<Response, ApiError> {
    let rd = service::patch_reading(&state, ctx.tenant_id()?, batch_id, reading_id, req).await?;
    Ok(Json(rd).into_response())
}

async fn delete(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((batch_id, reading_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    service::delete_reading(&state, ctx.tenant_id()?, batch_id, reading_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
