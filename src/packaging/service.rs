//! Packaging business logic: packaging-run lifecycle and distribution movements.
//!
//! Port of the Go `internal/packaging/service.go`. Mutating operations record a
//! compliance audit event (fire-and-forget). Outbound movements are checked
//! against remaining stock before being recorded.

use std::collections::BTreeMap;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use super::models::{
    CreateMovementRequest, CreatePackagingRunRequest, DistributionMovement, ListMovementsFilter,
    ListPackagingRunsFilter, PackagingRun, Page, PatchPackagingRunRequest, OUTBOUND_MOVEMENTS,
};
use super::repository as repo;
use crate::audit;
use crate::platform::errors::ApiError;
use crate::state::AppState;

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

// ---- packaging runs ----

/// Creates a packaging run for a batch.
pub async fn create_run(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    req: CreatePackagingRunRequest,
) -> Result<PackagingRun, ApiError> {
    let run = match repo::insert_run(
        &state.pool,
        tenant_id,
        req.batch_id,
        &req.format,
        req.unit_volume_ml,
        req.quantity,
        &req.lot_number,
        &req.packaged_at,
        req.best_before_date.as_deref(),
        req.notes.as_deref(),
    )
    .await
    {
        Ok(run) => run,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict("packaging_run", "lot_number"))
        }
        Err(e) => return Err(e.into()),
    };

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_PACKAGING_RUN_CREATED,
            entity_type: "packaging_run",
            entity_id: Some(run.id),
            actor_user_id: actor_id,
            event_data: json!({
                "batch_id": run.batch_id,
                "format": run.format,
                "lot_number": run.lot_number,
                "quantity": run.quantity,
                "unit_volume_ml": run.unit_volume_ml,
                "packaged_at": run.packaged_at,
            }),
        },
    )
    .await;
    Ok(run)
}

pub async fn get_run(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<PackagingRun, ApiError> {
    repo::select_run_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("packaging_run"))
}

pub async fn list_runs(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListPackagingRunsFilter,
) -> Result<Page<PackagingRun>, ApiError> {
    Ok(repo::select_runs(&state.pool, tenant_id, &filter).await?)
}

pub async fn patch_run(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchPackagingRunRequest,
) -> Result<PackagingRun, ApiError> {
    if req.best_before_date.is_none() && req.notes.is_none() {
        return Err(ApiError::validation("body", "at least one field required"));
    }
    let mut run = get_run(state, tenant_id, id).await?;
    if req.best_before_date.is_some() {
        run.best_before_date = req.best_before_date;
    }
    if let Some(notes) = req.notes {
        run.notes = Some(notes);
    }
    repo::update_run(
        &state.pool,
        tenant_id,
        id,
        run.best_before_date.as_deref(),
        run.notes.as_deref(),
    )
    .await?
    .ok_or_else(|| ApiError::not_found("packaging_run"))
}

/// Deletes a packaging run; blocked if it has movements.
pub async fn delete_run(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    id: Uuid,
) -> Result<(), ApiError> {
    if repo::has_movements(&state.pool, tenant_id, id).await? {
        return Err(ApiError::conflict("packaging_run", "has_movements"));
    }
    // Ensure the run exists (and is tenant-owned) before deleting.
    let run = get_run(state, tenant_id, id).await?;
    if !repo::delete_run(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("packaging_run"));
    }

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_PACKAGING_RUN_DELETED,
            entity_type: "packaging_run",
            entity_id: Some(id),
            actor_user_id: actor_id,
            event_data: json!({
                "batch_id": run.batch_id,
                "format": run.format,
                "lot_number": run.lot_number,
                "quantity": run.quantity,
            }),
        },
    )
    .await;
    Ok(())
}

// ---- distribution movements ----

/// Records a distribution movement, checking stock for outbound types.
pub async fn create_movement(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    req: CreateMovementRequest,
) -> Result<DistributionMovement, ApiError> {
    if req.movement_type == "sale" && req.order_id.is_none() {
        return Err(ApiError::validation(
            "order_id",
            "required for movement_type=sale",
        ));
    }

    if OUTBOUND_MOVEMENTS.contains(&req.movement_type.as_str()) {
        let stock = repo::stock_remaining(&state.pool, tenant_id, req.packaging_run_id)
            .await?
            .ok_or_else(|| ApiError::not_found("packaging_run"))?;
        if i64::from(req.quantity) > stock {
            let mut details = BTreeMap::new();
            details.insert("requested".to_string(), json!(req.quantity));
            details.insert("available".to_string(), json!(stock));
            return Err(ApiError::business_rule(
                "insufficient_stock",
                "Quantity exceeds available stock.",
                details,
            ));
        }
    }

    let from = match req.from_location {
        Some(ref f) if !f.is_empty() => f.clone(),
        _ => "brewery".to_string(),
    };
    let moved_at = req.moved_at.unwrap_or_else(Utc::now);

    let created = repo::insert_movement(
        &state.pool,
        tenant_id,
        req.packaging_run_id,
        &req.movement_type,
        req.quantity,
        &from,
        &req.to_location,
        req.order_id,
        req.reference.as_deref(),
        req.notes.as_deref(),
        moved_at,
    )
    .await?;

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_MOVEMENT_CREATED,
            entity_type: "distribution_movement",
            entity_id: Some(created.id),
            actor_user_id: actor_id,
            event_data: json!({
                "packaging_run_id": created.packaging_run_id,
                "movement_type": created.movement_type,
                "quantity": created.quantity,
                "to_location": created.to_location,
                "order_id": created.order_id,
            }),
        },
    )
    .await;
    Ok(created)
}

pub async fn get_movement(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<DistributionMovement, ApiError> {
    repo::select_movement_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("distribution_movement"))
}

pub async fn list_movements(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListMovementsFilter,
) -> Result<Page<DistributionMovement>, ApiError> {
    Ok(repo::select_movements(&state.pool, tenant_id, &filter).await?)
}

pub async fn delete_movement(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    id: Uuid,
) -> Result<(), ApiError> {
    // Ensure the movement exists (and is tenant-owned) before deleting.
    let m = get_movement(state, tenant_id, id).await?;
    if !repo::delete_movement(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("distribution_movement"));
    }

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_MOVEMENT_DELETED,
            entity_type: "distribution_movement",
            entity_id: Some(id),
            actor_user_id: actor_id,
            event_data: json!({
                "packaging_run_id": m.packaging_run_id,
                "movement_type": m.movement_type,
                "quantity": m.quantity,
                "to_location": m.to_location,
            }),
        },
    )
    .await;
    Ok(())
}
