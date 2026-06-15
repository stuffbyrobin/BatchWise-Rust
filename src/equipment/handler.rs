//! Equipment HTTP handlers and router (`/equipment`, `/maintenance-due`).
//!
//! Port of the Go equipment handlers. All routes require auth and the
//! `equipment_maintenance` feature flag (tier gate). The `/equipment` and
//! `/maintenance-due` groups are nested separately to avoid colliding with the
//! `/equipment/{id}` path parameter.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CreateEquipmentRequest, CreateEventRequest, CreateScheduleRequest, EventFilter, Filter,
    MaintenanceDueFilter, PatchEquipmentRequest, PatchScheduleRequest, ScheduleFilter,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the equipment routers (`/equipment`, `/maintenance-due`), gated by
/// auth + the `equipment_maintenance` feature flag.
pub fn routes(state: AppState) -> Router {
    let equipment = Router::new()
        .route("/", get(list_equipment).post(create_equipment))
        .route(
            "/{id}",
            get(get_equipment)
                .patch(patch_equipment)
                .delete(delete_equipment),
        )
        .route("/{id}/schedules", get(list_schedules).post(create_schedule))
        .route(
            "/{id}/schedules/{schedule_id}",
            patch(patch_schedule).delete(delete_schedule),
        )
        .route("/{id}/events", get(list_events).post(create_event))
        .route(
            "/{id}/events/{event_id}",
            axum::routing::delete(delete_event),
        );

    let maintenance_due = Router::new().route("/", get(list_maintenance_due));

    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "equipment_maintenance", req, next).await }
    });

    Router::new()
        .nest("/equipment", equipment)
        .nest("/maintenance-due", maintenance_due)
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct EquipmentQuery {
    status: Option<String>,
    equipment_type: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ScheduleQuery {
    active: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct EventQuery {
    event_type: Option<String>,
    schedule_id: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MaintenanceDueQuery {
    window_days: Option<String>,
    overdue_only: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

// ---- equipment ----

async fn list_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<EquipmentQuery>,
) -> Result<Response, ApiError> {
    let filter = Filter {
        status: q.status.filter(|s| !s.is_empty()),
        equipment_type: q.equipment_type.filter(|s| !s.is_empty()),
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
    };
    Ok(Json(service::list_equipment(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn create_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateEquipmentRequest>,
) -> Result<Response, ApiError> {
    let e = service::create_equipment(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/equipment/{}", e.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(e)).into_response())
}

async fn get_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_equipment(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchEquipmentRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_equipment(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_equipment(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_equipment(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- schedules ----

async fn list_schedules(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    Query(q): Query<ScheduleQuery>,
) -> Result<Response, ApiError> {
    let active = q
        .active
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<bool>().ok());
    let filter = ScheduleFilter {
        active,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
    };
    Ok(Json(service::list_schedules(&state, ctx.tenant_id()?, id, filter).await?).into_response())
}

async fn create_schedule(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateScheduleRequest>,
) -> Result<Response, ApiError> {
    let m = service::create_schedule(&state, ctx.tenant_id()?, id, req).await?;
    let location = format!("/api/v1/equipment/{}/schedules/{}", id, m.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(m)).into_response())
}

async fn patch_schedule(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((id, schedule_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(req): ValidatedJson<PatchScheduleRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::patch_schedule(&state, ctx.tenant_id()?, id, schedule_id, req).await?)
            .into_response(),
    )
}

async fn delete_schedule(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((id, schedule_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    service::delete_schedule(&state, ctx.tenant_id()?, id, schedule_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- events ----

async fn list_events(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    Query(q): Query<EventQuery>,
) -> Result<Response, ApiError> {
    let schedule_id = q
        .schedule_id
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(&s).ok());
    let filter = EventFilter {
        event_type: q.event_type.filter(|s| !s.is_empty()),
        schedule_id,
        sort: q.sort.unwrap_or_default(),
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
    };
    Ok(Json(service::list_events(&state, ctx.tenant_id()?, id, filter).await?).into_response())
}

async fn create_event(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateEventRequest>,
) -> Result<Response, ApiError> {
    let e = service::create_event(&state, ctx.tenant_id()?, id, req).await?;
    let location = format!("/api/v1/equipment/{}/events/{}", id, e.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(e)).into_response())
}

async fn delete_event(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((id, event_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    service::delete_event(&state, ctx.tenant_id()?, id, event_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- maintenance-due ----

async fn list_maintenance_due(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<MaintenanceDueQuery>,
) -> Result<Response, ApiError> {
    let window_days = q
        .window_days
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(30);
    let overdue_only = q
        .overdue_only
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let filter = MaintenanceDueFilter {
        window_days,
        overdue_only,
        page: q.page.unwrap_or(0),
        page_size: q.page_size.unwrap_or(0),
    };
    Ok(
        Json(service::list_maintenance_due(&state, ctx.tenant_id()?, filter).await?)
            .into_response(),
    )
}
