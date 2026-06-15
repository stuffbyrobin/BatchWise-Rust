//! Fermentation business logic: readings logged against a batch.
//!
//! Port of the Go `internal/fermentation/service.go`. Creating or listing
//! readings first verifies the batch belongs to the tenant (404 otherwise).

use chrono::Utc;
use uuid::Uuid;

use super::models::{CreateReadingRequest, Page, PatchReadingRequest, Reading, ReadingFilter};
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

pub async fn create_reading(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
    req: CreateReadingRequest,
) -> Result<Reading, ApiError> {
    if !repo::batch_exists(&state.pool, tenant_id, batch_id).await? {
        return Err(ApiError::not_found("batch"));
    }

    let recorded_at = req.recorded_at.unwrap_or_else(Utc::now);
    let stage = req.stage.as_deref().unwrap_or("primary");

    Ok(repo::insert_reading(
        &state.pool,
        tenant_id,
        batch_id,
        recorded_at,
        stage,
        req.gravity,
        req.temp_c,
        req.ph,
        req.notes.as_deref(),
    )
    .await?)
}

pub async fn list_readings(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
    filter: ReadingFilter,
) -> Result<Page<Reading>, ApiError> {
    if !repo::batch_exists(&state.pool, tenant_id, batch_id).await? {
        return Err(ApiError::not_found("batch"));
    }
    Ok(repo::select_readings(&state.pool, tenant_id, batch_id, &filter).await?)
}

pub async fn patch_reading(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
    id: Uuid,
    req: PatchReadingRequest,
) -> Result<Reading, ApiError> {
    let mut rd = repo::select_reading_by_id(&state.pool, tenant_id, batch_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("fermentation_reading"))?;

    if let Some(v) = req.recorded_at {
        rd.recorded_at = v;
    }
    if let Some(v) = req.stage {
        rd.stage = v;
    }
    if let Some(v) = req.gravity {
        rd.gravity = Some(v);
    }
    if let Some(v) = req.temp_c {
        rd.temp_c = Some(v);
    }
    if let Some(v) = req.ph {
        rd.ph = Some(v);
    }
    if let Some(v) = req.notes {
        rd.notes = Some(v);
    }

    if !repo::update_reading(
        &state.pool,
        tenant_id,
        batch_id,
        id,
        rd.recorded_at,
        &rd.stage,
        rd.gravity,
        rd.temp_c,
        rd.ph,
        rd.notes.as_deref(),
    )
    .await?
    {
        return Err(ApiError::not_found("fermentation_reading"));
    }
    Ok(rd)
}

pub async fn delete_reading(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
    id: Uuid,
) -> Result<(), ApiError> {
    if !repo::delete_reading(&state.pool, tenant_id, batch_id, id).await? {
        return Err(ApiError::not_found("fermentation_reading"));
    }
    Ok(())
}
