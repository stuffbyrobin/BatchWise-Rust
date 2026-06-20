//! Fermenter business logic: tenant-scoped CRUD.

use uuid::Uuid;

use super::models::{CreateRequest, Fermenter, FermenterWrite, ListFilter, Page, UpdateRequest};
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

fn is_unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.is_unique_violation())
}

fn write_from_create(req: &CreateRequest) -> FermenterWrite {
    FermenterWrite {
        name: req.name.clone(),
        capacity_liters: req.capacity_liters,
        notes: req.notes.clone(),
    }
}

/// Creates a fermenter. A duplicate name (per tenant) yields 409.
pub async fn create(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateRequest,
) -> Result<Fermenter, ApiError> {
    let w = write_from_create(&req);
    let mut tx = state.pool.begin().await?;
    let fermenter = match repo::insert(&mut *tx, tenant_id, &w).await {
        Ok(f) => f,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "name",
                "a fermenter with this name already exists",
            ))
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    Ok(fermenter)
}

/// Lists fermenters.
pub async fn list(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<Page<Fermenter>, ApiError> {
    let order_by = build_sort(&filter.sort);
    Ok(repo::select_list(&state.pool, tenant_id, &filter, &order_by).await?)
}

/// Fetches a single fermenter, returning 404 if absent.
pub async fn get(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Fermenter, ApiError> {
    repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("fermenter"))
}

/// Partially updates a fermenter.
pub async fn update(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: UpdateRequest,
) -> Result<Fermenter, ApiError> {
    let existing = repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("fermenter"))?;

    let mut w = FermenterWrite {
        name: existing.name,
        capacity_liters: existing.capacity_liters,
        notes: existing.notes,
    };
    if let Some(v) = req.name {
        w.name = v;
    }
    if req.capacity_liters.is_some() {
        w.capacity_liters = req.capacity_liters;
    }
    if req.notes.is_some() {
        w.notes = req.notes;
    }

    let mut tx = state.pool.begin().await?;
    let updated = match repo::update_full(&mut *tx, tenant_id, id, &w).await {
        Ok(Some(f)) => f,
        Ok(None) => return Err(ApiError::not_found("fermenter")),
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "name",
                "a fermenter with this name already exists",
            ))
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    Ok(updated)
}

/// Deletes a fermenter. Any assigned batches are detached (FK ON DELETE SET NULL).
pub async fn delete(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    if !repo::delete_by_id(&mut *tx, tenant_id, id).await? {
        return Err(ApiError::not_found("fermenter"));
    }
    tx.commit().await?;
    Ok(())
}

/// Builds a safe `ORDER BY` from the sort spec (default `name`).
fn build_sort(sort: &str) -> String {
    let spec = if sort.is_empty() { "name" } else { sort };
    let desc = spec.starts_with('-');
    let col = match spec.trim_start_matches('-') {
        "name" => "name",
        "capacity_liters" => "capacity_liters",
        "created_at" => "created_at",
        _ => "name",
    };
    format!("{col} {} NULLS LAST", if desc { "DESC" } else { "ASC" })
}
