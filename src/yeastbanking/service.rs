//! Yeast banking business logic: bank entries and propagation events.
//!
//! Port of the Go `internal/yeastbanking/service.go`. `days_in_storage` is
//! computed (not stored) after every fetch, mirroring `computeDaysInStorage`.

use std::collections::BTreeMap;

use chrono::Utc;
use uuid::Uuid;

use super::models::{
    compute_days_in_storage, CreatePropagationRequest, CreateYeastBankRequest, HarvestRequest,
    Page, PatchPropagationRequest, PatchYeastBankRequest, Propagation, YeastBankEntry,
    YeastBankFilter,
};
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

// ---- yeast bank entries ----

pub async fn create_entry(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateYeastBankRequest,
) -> Result<YeastBankEntry, ApiError> {
    let generation = req.generation.unwrap_or(1);
    let mut entry = repo::insert_entry(
        &state.pool,
        tenant_id,
        &req.name,
        req.library_yeast_id,
        generation,
        req.harvested_at,
        req.viability_percent,
        req.quantity_ml,
        req.storage_temp_c,
        req.location.as_deref(),
        "active",
        req.notes.as_deref(),
    )
    .await?;
    entry.days_in_storage = compute_days_in_storage(entry.harvested_at);
    Ok(entry)
}

pub async fn list_entries(
    state: &AppState,
    tenant_id: Uuid,
    filter: YeastBankFilter,
) -> Result<Page<YeastBankEntry>, ApiError> {
    let mut page = repo::select_entries(&state.pool, tenant_id, &filter).await?;
    for e in &mut page.items {
        e.days_in_storage = compute_days_in_storage(e.harvested_at);
    }
    Ok(page)
}

pub async fn get_entry(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<YeastBankEntry, ApiError> {
    let mut entry = repo::select_entry_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast_bank_entry"))?;
    entry.days_in_storage = compute_days_in_storage(entry.harvested_at);
    Ok(entry)
}

pub async fn patch_entry(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchYeastBankRequest,
) -> Result<YeastBankEntry, ApiError> {
    let mut entry = get_entry(state, tenant_id, id).await?;

    if let Some(status) = req.status {
        if entry.status == "discarded" {
            return Err(ApiError::business_rule(
                "discarded_terminal",
                "cannot change status of a discarded yeast bank entry",
                BTreeMap::new(),
            ));
        }
        entry.status = status;
    }
    if let Some(name) = req.name {
        entry.name = name;
    }
    if let Some(v) = req.library_yeast_id {
        entry.library_yeast_id = Some(v);
    }
    if let Some(v) = req.viability_percent {
        entry.viability_percent = Some(v);
    }
    if let Some(v) = req.quantity_ml {
        entry.quantity_ml = Some(v);
    }
    if let Some(v) = req.storage_temp_c {
        entry.storage_temp_c = Some(v);
    }
    if let Some(v) = req.location {
        entry.location = Some(v);
    }
    if let Some(v) = req.notes {
        entry.notes = Some(v);
    }

    repo::update_entry(
        &state.pool,
        tenant_id,
        id,
        &entry.name,
        entry.library_yeast_id,
        entry.generation,
        entry.harvested_at,
        entry.viability_percent,
        entry.quantity_ml,
        entry.storage_temp_c,
        entry.location.as_deref(),
        &entry.status,
        entry.notes.as_deref(),
    )
    .await?;
    entry.days_in_storage = compute_days_in_storage(entry.harvested_at);
    Ok(entry)
}

pub async fn delete_entry(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if !repo::delete_entry(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("yeast_bank_entry"));
    }
    Ok(())
}

pub async fn harvest(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: HarvestRequest,
) -> Result<YeastBankEntry, ApiError> {
    let mut entry = get_entry(state, tenant_id, id).await?;
    if entry.status == "discarded" {
        return Err(ApiError::business_rule(
            "discarded_terminal",
            "cannot harvest from a discarded yeast bank entry",
            BTreeMap::new(),
        ));
    }

    entry.generation += 1;
    entry.status = "active".to_string();
    entry.harvested_at = Some(req.harvested_at.unwrap_or_else(Utc::now));

    if let Some(v) = req.viability_percent {
        entry.viability_percent = Some(v);
    }
    if let Some(v) = req.quantity_ml {
        entry.quantity_ml = Some(v);
    }
    if let Some(v) = req.notes {
        entry.notes = Some(v);
    }

    repo::update_entry(
        &state.pool,
        tenant_id,
        id,
        &entry.name,
        entry.library_yeast_id,
        entry.generation,
        entry.harvested_at,
        entry.viability_percent,
        entry.quantity_ml,
        entry.storage_temp_c,
        entry.location.as_deref(),
        &entry.status,
        entry.notes.as_deref(),
    )
    .await?;
    entry.days_in_storage = compute_days_in_storage(entry.harvested_at);
    Ok(entry)
}

// ---- propagations ----

pub async fn create_propagation(
    state: &AppState,
    tenant_id: Uuid,
    bank_id: Uuid,
    req: CreatePropagationRequest,
) -> Result<Propagation, ApiError> {
    // Verify the bank entry exists for this tenant.
    repo::select_entry_by_id(&state.pool, tenant_id, bank_id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast_bank_entry"))?;

    let started_at = req.started_at.unwrap_or_else(Utc::now);
    Ok(repo::insert_propagation(
        &state.pool,
        tenant_id,
        bank_id,
        req.batch_id,
        started_at,
        req.completed_at,
        req.volume_ml,
        req.notes.as_deref(),
    )
    .await?)
}

pub async fn list_propagations(
    state: &AppState,
    tenant_id: Uuid,
    bank_id: Uuid,
    page: i64,
    page_size: i64,
) -> Result<Page<Propagation>, ApiError> {
    repo::select_entry_by_id(&state.pool, tenant_id, bank_id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast_bank_entry"))?;
    Ok(repo::select_propagations(&state.pool, tenant_id, bank_id, page, page_size).await?)
}

pub async fn patch_propagation(
    state: &AppState,
    tenant_id: Uuid,
    bank_id: Uuid,
    prop_id: Uuid,
    req: PatchPropagationRequest,
) -> Result<Propagation, ApiError> {
    repo::select_entry_by_id(&state.pool, tenant_id, bank_id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast_bank_entry"))?;
    let mut prop = repo::select_propagation_by_id(&state.pool, tenant_id, bank_id, prop_id)
        .await?
        .ok_or_else(|| ApiError::not_found("propagation"))?;

    if let Some(v) = req.started_at {
        prop.started_at = v;
    }
    if let Some(v) = req.completed_at {
        prop.completed_at = Some(v);
    }
    if let Some(v) = req.volume_ml {
        prop.volume_ml = Some(v);
    }
    if let Some(v) = req.batch_id {
        prop.batch_id = Some(v);
    }
    if let Some(v) = req.notes {
        prop.notes = Some(v);
    }

    repo::update_propagation(
        &state.pool,
        tenant_id,
        prop_id,
        prop.batch_id,
        prop.started_at,
        prop.completed_at,
        prop.volume_ml,
        prop.notes.as_deref(),
    )
    .await?;
    Ok(prop)
}

pub async fn delete_propagation(
    state: &AppState,
    tenant_id: Uuid,
    bank_id: Uuid,
    prop_id: Uuid,
) -> Result<(), ApiError> {
    repo::select_entry_by_id(&state.pool, tenant_id, bank_id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast_bank_entry"))?;
    repo::select_propagation_by_id(&state.pool, tenant_id, bank_id, prop_id)
        .await?
        .ok_or_else(|| ApiError::not_found("propagation"))?;
    repo::delete_propagation(&state.pool, tenant_id, prop_id).await?;
    Ok(())
}
