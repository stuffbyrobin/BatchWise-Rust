//! Compliance-audit HTTP handlers and router (`/compliance-audit`, read-only).
//!
//! Port of the Go `internal/compliance/audit/handler.go`. Both routes require
//! auth but are **not** feature-gated — the audit log is always readable.

use axum::extract::{Path, Query, State};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{AuditEvent, AuditEventList, ListFilter};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::state::AppState;

/// Builds the `/compliance-audit` router (auth required, no feature gate).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .nest(
            "/compliance-audit",
            Router::new()
                .route("/", get(list))
                .route("/{id}", get(get_one)),
        )
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    entity_type: Option<String>,
    event_type: Option<String>,
    entity_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort: Option<String>,
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.is_empty())
}

async fn list(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<ListQuery>,
) -> Result<Json<AuditEventList>, ApiError> {
    let entity_id = match non_empty(q.entity_id) {
        Some(s) => Some(
            Uuid::parse_str(&s)
                .map_err(|_| ApiError::validation("entity_id", "must be a valid UUID"))?,
        ),
        None => None,
    };
    let from = match non_empty(q.from) {
        Some(s) => Some(parse_rfc3339(&s, "from")?),
        None => None,
    };
    let to = match non_empty(q.to) {
        Some(s) => Some(parse_rfc3339(&s, "to")?),
        None => None,
    };

    let filter = ListFilter {
        entity_type: non_empty(q.entity_type),
        entity_id,
        event_type: non_empty(q.event_type),
        from,
        to,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(1),
        page_size: q.page_size.unwrap_or(50),
    };
    Ok(Json(
        service::list(&state.pool, ctx.tenant_id()?, filter).await?,
    ))
}

async fn get_one(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Json<AuditEvent>, ApiError> {
    Ok(Json(service::get(&state.pool, ctx.tenant_id()?, id).await?))
}

fn parse_rfc3339(s: &str, field: &'static str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| ApiError::validation(field, "must be RFC 3339"))
}
