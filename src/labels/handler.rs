//! Label-record HTTP handlers and router.
//!
//! Port of the Go `internal/compliance/labels/handler.go`. The Go module mounts
//! a single chi router at `/label-records`; the orchestrator nests this router
//! at that path. All routes require auth and the `labels` feature flag.
//!
//! Create returns `201 Created` with a `Location` header; delete returns
//! `204 No Content`; everything else renders JSON with `200 OK`.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{CreateRequest, ListFilter, PatchRequest};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the label-records router (mounted at `/label-records`), gated by auth +
/// the `labels` feature flag.
pub fn routes(state: AppState) -> Router {
    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "labels", req, next).await }
    });
    Router::new()
        .route("/", post(create).get(list))
        .route("/{id}", get(get_by_id).patch(patch).delete(delete))
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    batch_id: Option<String>,
    status: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn create(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateRequest>,
) -> Result<Response, ApiError> {
    let rec = service::create(&state, ctx.tenant_id()?, ctx.actor_id, req).await?;
    let location = format!("/api/v1/label-records/{}", rec.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(rec),
    )
        .into_response())
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let filter = ListFilter {
        // Match the Go handler: an unparseable batch_id is silently ignored.
        batch_id: q.batch_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        status: q.status,
        sort: q.sort,
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(20),
    };
    let page = service::list(&state, ctx.tenant_id()?, filter).await?;
    Ok(Json(page).into_response())
}

async fn get_by_id(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let rec = service::get(&state, ctx.tenant_id()?, id).await?;
    Ok(Json(rec).into_response())
}

async fn patch(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchRequest>,
) -> Result<Response, ApiError> {
    let rec = service::patch(&state, ctx.tenant_id()?, id, ctx.actor_id, req).await?;
    Ok(Json(rec).into_response())
}

async fn delete(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete(&state, ctx.tenant_id()?, ctx.actor_id, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
