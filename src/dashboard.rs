//! Aggregated dashboard stats across modules.
//!
//! Port of the Go `internal/dashboard` package. The dashboard service calls the
//! other modules' service functions (respecting module boundaries) rather than
//! touching their tables directly.

use std::collections::HashMap;

use axum::extract::State;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::require_auth;
use crate::state::AppState;

/// Counts for the (non-terminal + terminal) batch statuses shown on the dashboard.
#[derive(Debug, Serialize)]
pub struct BatchStatusBreakdown {
    pub planned: i64,
    pub brewing: i64,
    pub fermenting: i64,
    pub conditioning: i64,
    pub packaging: i64,
    pub completed: i64,
    pub cancelled: i64,
}

/// Dashboard aggregation response.
#[derive(Debug, Serialize)]
pub struct Stats {
    pub low_stock_count: i64,
    pub expiring_soon_count: i64,
    pub upcoming_events_count: i64,
    pub active_batches_count: i64,
    pub batch_status_breakdown: BatchStatusBreakdown,
    pub recipes_count: i64,
    pub generated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers_in_use_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers_empty_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_30d_estimated_duty_pence: Option<i64>,
}

/// Builds the dashboard router (mounted at `/dashboard`).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/stats", get(stats))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

async fn stats(State(state): State<AppState>, ctx: RequestContext) -> Result<Response, ApiError> {
    let tenant_id = ctx.tenant_id()?;
    Ok(Json(compute(&state, tenant_id).await?).into_response())
}

/// Aggregates the dashboard statistics for a tenant.
pub async fn compute(state: &AppState, tenant_id: Uuid) -> Result<Stats, ApiError> {
    use crate::{batch, calendar, inventory, recipe, reporting, tracking};

    let now = Utc::now();
    let flags = feature_flags(state, tenant_id).await?;

    let low_stock_count = inventory::service::count_low_stock(state, tenant_id).await?;

    let expiring_soon_count = inventory::service::list(
        state,
        tenant_id,
        inventory::models::ListFilter {
            expiring_within_days: Some(30),
            page: 1,
            page_size: 1,
            ..Default::default()
        },
    )
    .await?
    .total;

    let upcoming_events_count = calendar::service::count_upcoming_pending(
        state,
        tenant_id,
        now,
        now + chrono::Duration::days(7),
    )
    .await?;

    let bd = batch::service::status_breakdown(state, tenant_id).await?;
    let get = |k: &str| bd.get(k).copied().unwrap_or(0);
    let active_batches_count = get("planned")
        + get("brewing")
        + get("fermenting")
        + get("conditioning")
        + get("packaging");

    let recipes_count = recipe::service::list(
        state,
        tenant_id,
        recipe::models::ListFilter {
            page: 1,
            page_size: 1,
            ..Default::default()
        },
    )
    .await?
    .total;

    let mut stats = Stats {
        low_stock_count,
        expiring_soon_count,
        upcoming_events_count,
        active_batches_count,
        batch_status_breakdown: BatchStatusBreakdown {
            planned: get("planned"),
            brewing: get("brewing"),
            fermenting: get("fermenting"),
            conditioning: get("conditioning"),
            packaging: get("packaging"),
            completed: get("completed"),
            cancelled: get("cancelled"),
        },
        recipes_count,
        generated_at: now,
        containers_in_use_count: None,
        containers_empty_count: None,
        last_30d_estimated_duty_pence: None,
    };

    if flags.get("tracking").copied().unwrap_or(false) {
        stats.containers_in_use_count = Some(
            tracking::service::count_assets_by_statuses(
                state,
                tenant_id,
                &["filled".to_string(), "delivered".to_string()],
            )
            .await?,
        );
        stats.containers_empty_count = Some(
            tracking::service::count_assets_by_statuses(state, tenant_id, &["empty".to_string()])
                .await?,
        );
    }

    if flags.get("reporting").copied().unwrap_or(false) {
        stats.last_30d_estimated_duty_pence = Some(
            reporting::service::sum_recent_duty_pence(
                state,
                tenant_id,
                now - chrono::Duration::days(30),
            )
            .await?,
        );
    }

    Ok(stats)
}

async fn feature_flags(
    state: &AppState,
    tenant_id: Uuid,
) -> Result<HashMap<String, bool>, ApiError> {
    let flags: Option<sqlx::types::Json<HashMap<String, bool>>> =
        sqlx::query_scalar("SELECT feature_flags FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_optional(&state.pool)
            .await?;
    Ok(flags.map(|j| j.0).unwrap_or_default())
}
