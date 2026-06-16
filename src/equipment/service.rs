//! Equipment maintenance business logic.
//!
//! Port of the Go `internal/equipment/service.go`. After every create/patch the
//! row is re-read so the computed fields (overdue counts, next-due, lifetime
//! cost) are populated consistently.

use std::collections::BTreeMap;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use super::models::{
    CreateEquipmentRequest, CreateEventRequest, CreateScheduleRequest, Equipment, EventFilter,
    Filter, MaintenanceDueFilter, MaintenanceDueItem, MaintenanceEvent, MaintenanceSchedule, Page,
    PatchEquipmentRequest, PatchScheduleRequest, ScheduleFilter,
};
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

// ---- equipment ----

pub async fn create_equipment(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateEquipmentRequest,
) -> Result<Equipment, ApiError> {
    let id = repo::insert_equipment(
        &state.pool,
        tenant_id,
        &req.name,
        &req.equipment_type,
        req.serial_number.as_deref(),
        req.location.as_deref(),
        "active",
        req.purchased_at.as_deref(),
        req.notes.as_deref(),
    )
    .await?;
    // Re-read to populate computed fields (counts, next-due, lifetime cost).
    repo::select_equipment_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("equipment"))
}

pub async fn list_equipment(
    state: &AppState,
    tenant_id: Uuid,
    filter: Filter,
) -> Result<Page<Equipment>, ApiError> {
    Ok(repo::select_equipment(&state.pool, tenant_id, &filter).await?)
}

pub async fn get_equipment(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Equipment, ApiError> {
    repo::select_equipment_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("equipment"))
}

pub async fn patch_equipment(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchEquipmentRequest,
) -> Result<Equipment, ApiError> {
    let mut e = get_equipment(state, tenant_id, id).await?;
    if let Some(v) = req.name {
        e.name = v;
    }
    if let Some(v) = req.equipment_type {
        e.equipment_type = v;
    }
    if let Some(v) = req.serial_number {
        e.serial_number = Some(v);
    }
    if let Some(v) = req.location {
        e.location = Some(v);
    }
    if let Some(v) = req.status {
        e.status = v;
    }
    if let Some(v) = req.purchased_at {
        e.purchased_at = Some(v);
    }
    if let Some(v) = req.notes {
        e.notes = Some(v);
    }
    if !repo::update_equipment(
        &state.pool,
        tenant_id,
        id,
        &e.name,
        &e.equipment_type,
        e.serial_number.as_deref(),
        e.location.as_deref(),
        &e.status,
        e.purchased_at.as_deref(),
        e.notes.as_deref(),
    )
    .await?
    {
        return Err(ApiError::not_found("equipment"));
    }
    get_equipment(state, tenant_id, id).await
}

pub async fn delete_equipment(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if !repo::delete_equipment(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("equipment"));
    }
    Ok(())
}

// ---- schedules ----

pub async fn create_schedule(
    state: &AppState,
    tenant_id: Uuid,
    equipment_id: Uuid,
    req: CreateScheduleRequest,
) -> Result<MaintenanceSchedule, ApiError> {
    get_equipment(state, tenant_id, equipment_id).await?;
    let active = req.active.unwrap_or(true);
    let id = repo::insert_schedule(
        &state.pool,
        tenant_id,
        equipment_id,
        &req.task_name,
        req.interval_days,
        req.last_performed_at,
        active,
        req.notes.as_deref(),
    )
    .await?;
    repo::select_schedule_by_id(&state.pool, tenant_id, equipment_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("maintenance_schedule"))
}

pub async fn list_schedules(
    state: &AppState,
    tenant_id: Uuid,
    equipment_id: Uuid,
    filter: ScheduleFilter,
) -> Result<Page<MaintenanceSchedule>, ApiError> {
    get_equipment(state, tenant_id, equipment_id).await?;
    Ok(repo::select_schedules(&state.pool, tenant_id, equipment_id, &filter).await?)
}

pub async fn patch_schedule(
    state: &AppState,
    tenant_id: Uuid,
    equipment_id: Uuid,
    schedule_id: Uuid,
    req: PatchScheduleRequest,
) -> Result<MaintenanceSchedule, ApiError> {
    let mut m = repo::select_schedule_by_id(&state.pool, tenant_id, equipment_id, schedule_id)
        .await?
        .ok_or_else(|| ApiError::not_found("maintenance_schedule"))?;
    if let Some(v) = req.task_name {
        m.task_name = v;
    }
    if let Some(v) = req.interval_days {
        m.interval_days = v;
    }
    if let Some(v) = req.last_performed_at {
        m.last_performed_at = Some(v);
    }
    if let Some(v) = req.active {
        m.active = v;
    }
    if let Some(v) = req.notes {
        m.notes = Some(v);
    }
    if !repo::update_schedule(
        &state.pool,
        tenant_id,
        schedule_id,
        &m.task_name,
        m.interval_days,
        m.last_performed_at,
        m.active,
        m.notes.as_deref(),
    )
    .await?
    {
        return Err(ApiError::not_found("maintenance_schedule"));
    }
    repo::select_schedule_by_id(&state.pool, tenant_id, equipment_id, schedule_id)
        .await?
        .ok_or_else(|| ApiError::not_found("maintenance_schedule"))
}

pub async fn delete_schedule(
    state: &AppState,
    tenant_id: Uuid,
    equipment_id: Uuid,
    schedule_id: Uuid,
) -> Result<(), ApiError> {
    repo::select_schedule_by_id(&state.pool, tenant_id, equipment_id, schedule_id)
        .await?
        .ok_or_else(|| ApiError::not_found("maintenance_schedule"))?;
    repo::delete_schedule(&state.pool, tenant_id, schedule_id).await?;
    Ok(())
}

// ---- events ----

pub async fn create_event(
    state: &AppState,
    tenant_id: Uuid,
    equipment_id: Uuid,
    req: CreateEventRequest,
) -> Result<MaintenanceEvent, ApiError> {
    get_equipment(state, tenant_id, equipment_id).await?;

    if let Some(schedule_id) = req.schedule_id {
        let sched = repo::select_schedule_for_tenant(&state.pool, tenant_id, schedule_id)
            .await?
            .ok_or_else(|| ApiError::not_found("maintenance_schedule"))?;
        if sched.equipment_id != equipment_id {
            let mut details = BTreeMap::new();
            details.insert("schedule_id".to_string(), json!(sched.id.to_string()));
            details.insert(
                "schedule_equipment_id".to_string(),
                json!(sched.equipment_id.to_string()),
            );
            details.insert("equipment_id".to_string(), json!(equipment_id.to_string()));
            return Err(ApiError::business_rule(
                "schedule_equipment_mismatch",
                "the schedule belongs to different equipment",
                details,
            ));
        }
    }

    let performed_at = req.performed_at.unwrap_or_else(Utc::now);
    let currency = req.cost_currency.unwrap_or_else(|| "GBP".to_string());

    let event = repo::insert_event(
        &state.pool,
        tenant_id,
        equipment_id,
        req.schedule_id,
        &req.event_type,
        performed_at,
        req.performed_by.as_deref(),
        req.cost_pence,
        &currency,
        req.notes.as_deref(),
    )
    .await?;

    // Logging against a schedule advances its last_performed_at, but only forward.
    if let Some(schedule_id) = req.schedule_id {
        repo::advance_schedule_last_performed(&state.pool, tenant_id, schedule_id, performed_at)
            .await?;
    }
    Ok(event)
}

pub async fn list_events(
    state: &AppState,
    tenant_id: Uuid,
    equipment_id: Uuid,
    filter: EventFilter,
) -> Result<Page<MaintenanceEvent>, ApiError> {
    get_equipment(state, tenant_id, equipment_id).await?;
    Ok(repo::select_events(&state.pool, tenant_id, equipment_id, &filter).await?)
}

pub async fn delete_event(
    state: &AppState,
    tenant_id: Uuid,
    equipment_id: Uuid,
    event_id: Uuid,
) -> Result<(), ApiError> {
    repo::select_event_by_id(&state.pool, tenant_id, equipment_id, event_id)
        .await?
        .ok_or_else(|| ApiError::not_found("maintenance_event"))?;
    repo::delete_event(&state.pool, tenant_id, event_id).await?;
    Ok(())
}

// ---- maintenance due feed ----

pub async fn list_maintenance_due(
    state: &AppState,
    tenant_id: Uuid,
    filter: MaintenanceDueFilter,
) -> Result<Page<MaintenanceDueItem>, ApiError> {
    Ok(repo::select_maintenance_due(&state.pool, tenant_id, &filter).await?)
}
