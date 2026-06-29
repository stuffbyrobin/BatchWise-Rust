//! Water business logic.
//!
//! Port of the Go `internal/water/service.go`. Profile reads return the union of
//! the system tenant and the caller's tenant; writes target only the caller's
//! tenant, and attempts to modify a system-tenant (or non-existent) profile
//! resolve to `not_found`. Saving an adjustment (create/update/patch) always
//! recomputes the cached result via [`crate::pkg::water`].

use uuid::Uuid;

use super::models::{
    AcidAddition, Adjustment, AdjustmentFilter, CalculateRequest, CreateWaterAdjustmentRequest,
    CreateWaterProfileRequest, GrainAddition, MineralAddition, Page, PatchWaterAdjustmentRequest,
    PatchWaterProfileRequest, Profile, ProfileFilter, Result as WaterResult,
    UpdateWaterAdjustmentRequest, UpdateWaterProfileRequest,
};
use super::repository as repo;
use crate::pkg::water as pw;
use crate::platform::errors::ApiError;
use crate::state::AppState;

// ---- Water profiles ----

/// Creates a water profile owned by the caller.
pub async fn create_water_profile(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateWaterProfileRequest,
) -> Result<Profile, ApiError> {
    repo::insert_water_profile(&state.pool, tenant_id, &req).await
}

/// Fetches one water profile visible to the caller.
pub async fn get_water_profile(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Profile, ApiError> {
    repo::select_water_profile(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("water_profile"))
}

/// Lists water profiles visible to the caller.
pub async fn list_water_profiles(
    state: &AppState,
    tenant_id: Uuid,
    filter: ProfileFilter,
) -> Result<Page<Profile>, ApiError> {
    repo::select_water_profiles(&state.pool, tenant_id, &filter).await
}

/// Replaces a caller-owned, non-system water profile.
pub async fn update_water_profile(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: UpdateWaterProfileRequest,
) -> Result<Profile, ApiError> {
    let existing = repo::select_water_profile(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("water_profile"))?;
    if existing.is_system {
        return Err(ApiError::not_found("water_profile"));
    }
    repo::update_water_profile(&state.pool, tenant_id, id, &req)
        .await?
        .ok_or_else(|| ApiError::not_found("water_profile"))
}

/// Partially updates a caller-owned, non-system water profile.
pub async fn patch_water_profile(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchWaterProfileRequest,
) -> Result<Profile, ApiError> {
    let mut existing = repo::select_water_profile(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("water_profile"))?;
    if existing.is_system {
        return Err(ApiError::not_found("water_profile"));
    }

    if let Some(v) = req.name {
        existing.name = v;
    }
    if req.description.is_some() {
        existing.description = req.description;
    }
    if let Some(v) = req.calcium_ppm {
        existing.calcium_ppm = v;
    }
    if let Some(v) = req.magnesium_ppm {
        existing.magnesium_ppm = v;
    }
    if let Some(v) = req.sodium_ppm {
        existing.sodium_ppm = v;
    }
    if let Some(v) = req.sulfate_ppm {
        existing.sulfate_ppm = v;
    }
    if let Some(v) = req.chloride_ppm {
        existing.chloride_ppm = v;
    }
    if let Some(v) = req.bicarbonate_ppm {
        existing.bicarbonate_ppm = v;
    }
    if req.notes.is_some() {
        existing.notes = req.notes;
    }

    let merged = CreateWaterProfileRequest {
        name: existing.name,
        description: existing.description,
        calcium_ppm: existing.calcium_ppm,
        magnesium_ppm: existing.magnesium_ppm,
        sodium_ppm: existing.sodium_ppm,
        sulfate_ppm: existing.sulfate_ppm,
        chloride_ppm: existing.chloride_ppm,
        bicarbonate_ppm: existing.bicarbonate_ppm,
        notes: existing.notes,
    };
    repo::update_water_profile(&state.pool, tenant_id, id, &merged)
        .await?
        .ok_or_else(|| ApiError::not_found("water_profile"))
}

/// Deletes a caller-owned, non-system water profile.
pub async fn delete_water_profile(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<(), ApiError> {
    let existing = repo::select_water_profile(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("water_profile"))?;
    if existing.is_system {
        return Err(ApiError::not_found("water_profile"));
    }
    if repo::delete_water_profile(&state.pool, tenant_id, id).await? == 0 {
        return Err(ApiError::not_found("water_profile"));
    }
    Ok(())
}

// ---- Calculate ----

/// Stateless water-treatment calculation. Requires a source profile (by id or
/// inline).
pub async fn calculate(
    state: &AppState,
    tenant_id: Uuid,
    req: CalculateRequest,
) -> Result<WaterResult, ApiError> {
    compute_result(state, tenant_id, &req, true).await
}

/// Resolves the source profile, applies the additions via [`crate::pkg::water`],
/// and returns the cached [`WaterResult`]. When `strict` is false an absent
/// source resolves to distilled (zero) water; when true it is required.
async fn compute_result(
    state: &AppState,
    tenant_id: Uuid,
    req: &CalculateRequest,
    strict: bool,
) -> Result<WaterResult, ApiError> {
    let has_id = req.source_profile_id.is_some();
    let has_inline = req.source_profile.is_some();

    if has_id && has_inline {
        return Err(ApiError::validation(
            "source",
            "provide either source_profile_id or source_profile, not both",
        ));
    }
    if strict && !has_id && !has_inline {
        return Err(ApiError::validation(
            "source",
            "provide either source_profile_id or source_profile",
        ));
    }

    let source = if let Some(source_id) = req.source_profile_id {
        let p = repo::select_water_profile(&state.pool, tenant_id, source_id)
            .await?
            .ok_or_else(|| ApiError::not_found("water_profile"))?;
        pw::WaterProfile {
            calcium: p.calcium_ppm,
            magnesium: p.magnesium_ppm,
            sodium: p.sodium_ppm,
            sulfate: p.sulfate_ppm,
            chloride: p.chloride_ppm,
            bicarbonate: p.bicarbonate_ppm,
        }
    } else if let Some(inline) = &req.source_profile {
        pw::WaterProfile {
            calcium: inline.calcium_ppm,
            magnesium: inline.magnesium_ppm,
            sodium: inline.sodium_ppm,
            sulfate: inline.sulfate_ppm,
            chloride: inline.chloride_ppm,
            bicarbonate: inline.bicarbonate_ppm,
        }
    } else {
        // Distilled water; only reachable when strict = false.
        pw::WaterProfile::default()
    };

    let minerals = map_minerals(&req.mineral_additions)?;
    let acids = map_acids(&req.acid_additions)?;

    let mash = if req.grain_additions.is_empty() {
        None
    } else {
        Some(pw::MashParameters {
            water_volume: req.volume_liters,
            grains: map_grains(&req.grain_additions),
            acids: acids.clone(),
        })
    };

    let calc = pw::calculate_water_treatment(source, req.volume_liters, &minerals, mash.as_ref())
        .map_err(|e| ApiError::internal(format!("calculate water treatment: {e}")))?;

    let mash_ph = if mash.is_some() { calc.mash_ph } else { 0.0 };

    Ok(WaterResult {
        calcium_ppm: calc.final_profile.calcium,
        magnesium_ppm: calc.final_profile.magnesium,
        sodium_ppm: calc.final_profile.sodium,
        sulfate_ppm: calc.final_profile.sulfate,
        chloride_ppm: calc.final_profile.chloride,
        bicarbonate_ppm: calc.final_profile.bicarbonate,
        alkalinity: calc.alkalinity,
        residual_alk: calc.residual_alk,
        sulfate_to_chloride: calc.sulfate_to_chloride,
        mash_ph,
    })
}

/// Maps mineral additions to `pkg/water` types. Unknown salts are an error,
/// mirroring the Go `applyMineral` default branch.
fn map_minerals(additions: &[MineralAddition]) -> Result<Vec<pw::MineralAddition>, ApiError> {
    additions
        .iter()
        .map(|m| {
            let mineral_type = match m.r#type.as_str() {
                "CaSO4" => pw::MineralType::Gypsum,
                "CaCl2" => pw::MineralType::CalciumCl,
                "CaCO3" => pw::MineralType::Chalk,
                "MgSO4" => pw::MineralType::Epsom,
                "MgCl2" => pw::MineralType::MagnesiumCl,
                "NaHCO3" => pw::MineralType::BakingSoda,
                "NaCl" => pw::MineralType::TableSalt,
                "Na2SO4" => pw::MineralType::SodiumSulfate,
                // The frontend sends "Ca(OH)2"; accept the bare form too.
                "CaOH2" | "Ca(OH)2" => pw::MineralType::SlakedLime,
                other => {
                    return Err(ApiError::internal(format!(
                        "calculate water treatment: unknown mineral type: {other:?}"
                    )))
                }
            };
            let form = match m.form.as_deref() {
                Some("anhydrous") => pw::MineralForm::Anhydrous,
                Some("liquid") => pw::MineralForm::Liquid,
                // "hydrate" (and the legacy "dihydrate") → the standard hydrate.
                Some("hydrate") | Some("dihydrate") => pw::MineralForm::Hydrate,
                // Form omitted (legacy data): fall back to each salt's historical
                // default — anhydrous for Na2SO4, the hydrate for everything else.
                _ => {
                    if mineral_type == pw::MineralType::SodiumSulfate {
                        pw::MineralForm::Anhydrous
                    } else {
                        pw::MineralForm::Hydrate
                    }
                }
            };
            Ok(pw::MineralAddition {
                mineral_type,
                amount: m.amount,
                form,
                strength_pct: m.strength_pct.unwrap_or(0.0),
            })
        })
        .collect()
}

/// Maps acid additions to `pkg/water` types. Unknown acids map to
/// [`pw::AcidType::None`], which errors during calculation (mirroring the Go
/// `acidProperties` default branch).
fn map_acids(additions: &[AcidAddition]) -> Result<Vec<pw::AcidAddition>, ApiError> {
    Ok(additions
        .iter()
        .map(|a| {
            let acid_type = match a.r#type.as_str() {
                "phosphoric" => pw::AcidType::Phosphoric,
                "lactic" => pw::AcidType::Lactic,
                "sulfuric" => pw::AcidType::Sulfuric,
                "hydrochloric" => pw::AcidType::Hydrochloric,
                _ => pw::AcidType::None,
            };
            pw::AcidAddition {
                acid_type,
                strength: a.strength,
                amount: a.amount,
            }
        })
        .collect())
}

/// Maps grain additions to `pkg/water` types. Unknown grain types are dropped
/// (they contribute zero acidity, matching the Go `grainContrib` default branch).
fn map_grains(additions: &[GrainAddition]) -> Vec<pw::GrainAddition> {
    additions
        .iter()
        .filter_map(|g| {
            let grain_type = match g.r#type.as_str() {
                "base" => pw::GrainType::Base,
                "crystal" => pw::GrainType::Crystal,
                "roast" => pw::GrainType::Roast,
                "acid" => pw::GrainType::Acid,
                _ => return None,
            };
            Some(pw::GrainAddition {
                name: g.name.clone(),
                grain_type,
                weight: g.weight,
                colour: g.colour,
            })
        })
        .collect()
}

// ---- Water adjustments ----

/// Creates a water adjustment, computing and caching its result.
pub async fn create_water_adjustment(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateWaterAdjustmentRequest,
) -> Result<Adjustment, ApiError> {
    let calc_req = CalculateRequest {
        source_profile_id: req.source_profile_id,
        source_profile: None,
        volume_liters: req.volume_liters,
        mineral_additions: req.mineral_additions.clone(),
        acid_additions: req.acid_additions.clone(),
        grain_additions: req.grain_additions.clone(),
    };
    let result = compute_result(state, tenant_id, &calc_req, false).await?;
    repo::insert_water_adjustment(&state.pool, tenant_id, &req, Some(&result)).await
}

/// Fetches one caller-owned water adjustment.
pub async fn get_water_adjustment(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Adjustment, ApiError> {
    repo::select_water_adjustment(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("water_adjustment"))
}

/// Lists caller-owned water adjustments.
pub async fn list_water_adjustments(
    state: &AppState,
    tenant_id: Uuid,
    filter: AdjustmentFilter,
) -> Result<Page<Adjustment>, ApiError> {
    repo::select_water_adjustments(&state.pool, tenant_id, &filter).await
}

/// Replaces a caller-owned water adjustment, recomputing its result.
pub async fn update_water_adjustment(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: UpdateWaterAdjustmentRequest,
) -> Result<Adjustment, ApiError> {
    let calc_req = CalculateRequest {
        source_profile_id: req.source_profile_id,
        source_profile: None,
        volume_liters: req.volume_liters,
        mineral_additions: req.mineral_additions.clone(),
        acid_additions: req.acid_additions.clone(),
        grain_additions: req.grain_additions.clone(),
    };
    let result = compute_result(state, tenant_id, &calc_req, false).await?;
    repo::update_water_adjustment(&state.pool, tenant_id, id, &req, Some(&result))
        .await?
        .ok_or_else(|| ApiError::not_found("water_adjustment"))
}

/// Partially updates a caller-owned water adjustment, always recomputing its
/// result from the merged state.
pub async fn patch_water_adjustment(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchWaterAdjustmentRequest,
) -> Result<Adjustment, ApiError> {
    let existing = repo::select_water_adjustment(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("water_adjustment"))?;

    // Merge patch fields onto the existing adjustment.
    let name = req.name.unwrap_or(existing.name);
    let source_profile_id = if req.source_profile_id.is_some() {
        req.source_profile_id
    } else {
        existing.source_profile_id
    };
    let target_profile_id = if req.target_profile_id.is_some() {
        req.target_profile_id
    } else {
        existing.target_profile_id
    };
    let batch_id = if req.batch_id.is_some() {
        req.batch_id
    } else {
        existing.batch_id
    };
    let recipe_id = if req.recipe_id.is_some() {
        req.recipe_id
    } else {
        existing.recipe_id
    };
    let volume_liters = req.volume_liters.unwrap_or(existing.volume_liters);
    let mineral_additions = req.mineral_additions.unwrap_or(existing.mineral_additions);
    let acid_additions = req.acid_additions.unwrap_or(existing.acid_additions);
    let grain_additions = req.grain_additions.unwrap_or(existing.grain_additions);
    let notes = if req.notes.is_some() {
        req.notes
    } else {
        existing.notes
    };

    // Always recompute the cached result.
    let calc_req = CalculateRequest {
        source_profile_id,
        source_profile: None,
        volume_liters,
        mineral_additions: mineral_additions.clone(),
        acid_additions: acid_additions.clone(),
        grain_additions: grain_additions.clone(),
    };
    let result = compute_result(state, tenant_id, &calc_req, false).await?;

    let merged = UpdateWaterAdjustmentRequest {
        name,
        source_profile_id,
        target_profile_id,
        batch_id,
        recipe_id,
        volume_liters,
        mineral_additions,
        acid_additions,
        grain_additions,
        notes,
    };
    repo::update_water_adjustment(&state.pool, tenant_id, id, &merged, Some(&result))
        .await?
        .ok_or_else(|| ApiError::not_found("water_adjustment"))
}

/// Deletes a caller-owned water adjustment.
pub async fn delete_water_adjustment(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<(), ApiError> {
    if repo::delete_water_adjustment(&state.pool, tenant_id, id).await? == 0 {
        return Err(ApiError::not_found("water_adjustment"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_minerals_accepts_both_slaked_lime_spellings() {
        for ty in ["CaOH2", "Ca(OH)2"] {
            let out = map_minerals(&[MineralAddition {
                r#type: ty.to_string(),
                amount: 5.0,
                form: None,
                strength_pct: None,
            }])
            .unwrap_or_else(|e| panic!("{ty} should map: {e:?}"));
            assert_eq!(out[0].mineral_type, pw::MineralType::SlakedLime);
        }
    }
}
