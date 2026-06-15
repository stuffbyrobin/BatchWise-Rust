//! Yeast kinetics business logic.
//!
//! Port of the Go `internal/yeastkinetics/service.go`. Writes are tenant-scoped;
//! reads by id are tenant-scoped (a cross-tenant id reads as not-found). Yeast
//! existence is checked against the caller's tenant or the shared system tenant.

use uuid::Uuid;

use super::models::{CreateRequest, Kinetics, ListFilter, Page, PatchRequest};
use super::repository::{self as repo, KineticsWrite};
use crate::platform::errors::ApiError;
use crate::state::AppState;

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

fn write_from_create(req: &CreateRequest) -> KineticsWrite {
    KineticsWrite {
        yeast_id: req.yeast_id,
        fermentation_temp_c: req.fermentation_temp_c,
        primary_fermentation_days: req.primary_fermentation_days,
        conditioning_days: req.conditioning_days,
        lag_phase_hours: req.lag_phase_hours,
        attenuation_pct: req.attenuation_pct,
        notes: req.notes.clone(),
    }
}

fn write_from_existing(k: &Kinetics) -> KineticsWrite {
    KineticsWrite {
        yeast_id: k.yeast_id,
        fermentation_temp_c: k.fermentation_temp_c,
        primary_fermentation_days: k.primary_fermentation_days,
        conditioning_days: k.conditioning_days,
        lag_phase_hours: k.lag_phase_hours,
        attenuation_pct: k.attenuation_pct,
        notes: k.notes.clone(),
    }
}

/// Creates a yeast kinetics entry. Validates the yeast is accessible first.
pub async fn create(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateRequest,
) -> Result<Kinetics, ApiError> {
    if !repo::yeast_exists(&state.pool, tenant_id, req.yeast_id).await? {
        return Err(ApiError::validation(
            "yeast_id",
            "yeast not found or not accessible",
        ));
    }

    let w = write_from_create(&req);
    let mut tx = state.pool.begin().await?;
    let k = match repo::insert(&mut *tx, tenant_id, &w).await {
        Ok(k) => k,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "yeast_kinetics",
                "duplicate yeast_id+fermentation_temp_c",
            ));
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    Ok(k)
}

/// Lists yeast kinetics entries.
pub async fn list(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<Page<Kinetics>, ApiError> {
    Ok(repo::select_list(&state.pool, tenant_id, &filter).await?)
}

/// Fetches a yeast kinetics entry by id, tenant-scoped.
pub async fn get(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Kinetics, ApiError> {
    repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast kinetics"))
}

/// Replaces a yeast kinetics entry (PUT).
pub async fn replace(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: CreateRequest,
) -> Result<Kinetics, ApiError> {
    if !repo::yeast_exists(&state.pool, tenant_id, req.yeast_id).await? {
        return Err(ApiError::validation(
            "yeast_id",
            "yeast not found or not accessible",
        ));
    }

    if repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .is_none()
    {
        return Err(ApiError::not_found("yeast kinetics"));
    }

    let w = write_from_create(&req);
    let mut tx = state.pool.begin().await?;
    let k = match repo::update(&mut *tx, tenant_id, id, &w).await {
        Ok(Some(k)) => k,
        Ok(None) => return Err(ApiError::not_found("yeast kinetics")),
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "yeast_kinetics",
                "duplicate yeast_id+fermentation_temp_c",
            ));
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    Ok(k)
}

/// Partially updates a yeast kinetics entry.
pub async fn patch(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchRequest,
) -> Result<Kinetics, ApiError> {
    let existing = repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast kinetics"))?;

    let mut w = write_from_existing(&existing);
    if let Some(v) = req.yeast_id {
        if !repo::yeast_exists(&state.pool, tenant_id, v).await? {
            return Err(ApiError::validation(
                "yeast_id",
                "yeast not found or not accessible",
            ));
        }
        w.yeast_id = v;
    }
    if let Some(v) = req.fermentation_temp_c {
        w.fermentation_temp_c = v;
    }
    if let Some(v) = req.primary_fermentation_days {
        w.primary_fermentation_days = v;
    }
    if let Some(v) = req.conditioning_days {
        w.conditioning_days = v;
    }
    if req.lag_phase_hours.is_some() {
        w.lag_phase_hours = req.lag_phase_hours;
    }
    if req.attenuation_pct.is_some() {
        w.attenuation_pct = req.attenuation_pct;
    }
    if req.notes.is_some() {
        w.notes = req.notes;
    }

    let mut tx = state.pool.begin().await?;
    let k = match repo::update(&mut *tx, tenant_id, id, &w).await {
        Ok(Some(k)) => k,
        Ok(None) => return Err(ApiError::not_found("yeast kinetics")),
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "yeast_kinetics",
                "duplicate yeast_id+fermentation_temp_c",
            ));
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    Ok(k)
}

/// Deletes a yeast kinetics entry, returning 404 if absent.
pub async fn delete(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .is_none()
    {
        return Err(ApiError::not_found("yeast kinetics"));
    }
    let mut tx = state.pool.begin().await?;
    if !repo::delete_by_id(&mut *tx, tenant_id, id).await? {
        return Err(ApiError::not_found("yeast kinetics"));
    }
    tx.commit().await?;
    Ok(())
}

/// Finds the kinetics row for `yeast_id` whose `fermentation_temp_c` is closest
/// to `preferred_temp_c`, scoped to the caller's tenant (matches the Go repo,
/// which does not fall back to the system tenant). Returns `None` when there is
/// no matching row. Used by the batch module.
pub async fn find_closest_for_yeast(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    yeast_id: Uuid,
    preferred_temp_c: f64,
) -> Result<Option<Kinetics>, ApiError> {
    Ok(repo::find_closest_for_yeast(pool, tenant_id, yeast_id, preferred_temp_c).await?)
}
