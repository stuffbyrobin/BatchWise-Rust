//! Calendar event business logic: CRUD plus batch-generated event creation.
//!
//! Port of the Go `internal/calendar/service.go`.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::models::{CreateRequest, Event, EventWrite, ListFilter, Page, UpdateRequest};
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

fn write_from_create(req: &CreateRequest) -> EventWrite {
    EventWrite {
        batch_id: req.batch_id,
        event_type: req.event_type.clone(),
        title: req.title.clone(),
        start_time: req.start_time,
        end_time: req.end_time,
        status: req
            .status
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "pending".to_string()),
        notify_minutes_before: req.notify_minutes_before,
        notes: req.notes.clone(),
    }
}

/// Creates a calendar event.
pub async fn create(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateRequest,
) -> Result<Event, ApiError> {
    let w = write_from_create(&req);
    let mut tx = state.pool.begin().await?;
    let event = repo::insert(&mut *tx, tenant_id, &w).await?;
    tx.commit().await?;
    Ok(event)
}

/// Inserts a batch's generated events inside the caller's transaction.
///
/// Each event defaults to `pending` status when blank. Returns the created rows.
pub async fn create_for_batch_tx(
    conn: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    events: &[EventWrite],
) -> Result<Vec<Event>, ApiError> {
    let mut created = Vec::with_capacity(events.len());
    for e in events {
        let mut w = e.clone();
        if w.status.is_empty() {
            w.status = "pending".to_string();
        }
        let inserted = repo::insert(&mut *conn, tenant_id, &w).await?;
        created.push(inserted);
    }
    Ok(created)
}

/// Lists events.
pub async fn list(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<Page<Event>, ApiError> {
    let order_by = build_sort(&filter.sort);
    Ok(repo::select_list(&state.pool, tenant_id, &filter, &order_by).await?)
}

/// Fetches a single event, returning 404 if absent.
pub async fn get(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Event, ApiError> {
    repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("event"))
}

/// Partially updates an event.
pub async fn update(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: UpdateRequest,
) -> Result<Event, ApiError> {
    let existing = repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("event"))?;

    let mut w = EventWrite {
        batch_id: existing.batch_id,
        event_type: existing.event_type,
        title: existing.title,
        start_time: existing.start_time,
        end_time: existing.end_time,
        status: existing.status,
        notify_minutes_before: existing.notify_minutes_before,
        notes: existing.notes,
    };
    if req.batch_id.is_some() {
        w.batch_id = req.batch_id;
    }
    if let Some(v) = req.event_type {
        w.event_type = v;
    }
    if let Some(v) = req.title {
        w.title = v;
    }
    if let Some(v) = req.start_time {
        w.start_time = v;
    }
    if req.end_time.is_some() {
        w.end_time = req.end_time;
    }
    if let Some(v) = req.status {
        w.status = v;
    }
    if req.notify_minutes_before.is_some() {
        w.notify_minutes_before = req.notify_minutes_before;
    }
    if req.notes.is_some() {
        w.notes = req.notes;
    }

    let mut tx = state.pool.begin().await?;
    let updated = repo::update_full(&mut *tx, tenant_id, id, &w)
        .await?
        .ok_or_else(|| ApiError::not_found("event"))?;
    tx.commit().await?;
    Ok(updated)
}

/// Deletes an event. Only `custom` events may be deleted.
pub async fn delete(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let existing = repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("event"))?;

    if existing.event_type != "custom" {
        return Err(ApiError::business_rule(
            "auto_generated_event_not_deletable",
            "Auto-generated events cannot be deleted. Mark them completed or skipped instead.",
            Default::default(),
        ));
    }

    let mut tx = state.pool.begin().await?;
    if !repo::delete_by_id(&mut *tx, tenant_id, id).await? {
        return Err(ApiError::not_found("event"));
    }
    tx.commit().await?;
    Ok(())
}

/// Counts pending events starting in the (from, to] range (dashboard helper).
pub async fn count_upcoming_pending(
    state: &AppState,
    tenant_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<i64, ApiError> {
    Ok(repo::count_pending_for_range(&state.pool, tenant_id, from, to).await?)
}

/// Builds a safe `ORDER BY` from the sort spec (default `start_time`).
fn build_sort(sort: &str) -> String {
    let spec = if sort.is_empty() { "start_time" } else { sort };
    let desc = spec.starts_with('-');
    let col = spec.trim_start_matches('-');
    let mapped = match col {
        "start_time" => "start_time",
        "event_type" => "event_type",
        "title" => "title",
        "status" => "status",
        _ => "start_time",
    };
    format!("{mapped} {}", if desc { "DESC" } else { "ASC" })
}
