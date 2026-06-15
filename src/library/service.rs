//! Library business logic.
//!
//! Port of the Go `internal/library/service.go`. Reads return the union of the
//! system tenant and the caller's tenant; writes target only the caller's
//! tenant. Attempts to modify a system-tenant (or non-existent) row resolve to
//! `not_found` (cross-tenant is never surfaced as `forbidden`).

use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    EquipmentFilter, EquipmentProfile, EquipmentRequest, Fermentable, FermentableFilter,
    FermentableRequest, MashFilter, MashProfile, MashProfileRequest, MashStepRequest, Page,
    PatchEquipmentRequest, PatchFermentableRequest, PatchMashProfileRequest, PatchStyleRequest,
    PatchYeastRequest, Style, StyleFilter, StyleRequest, Yeast, YeastFilter, YeastRequest,
};
use super::repository as repo;
use crate::platform::errors::ApiError;

// ---- Styles ----

/// Creates a style owned by the caller.
pub async fn create_style(
    pool: &PgPool,
    tenant_id: Uuid,
    req: StyleRequest,
) -> Result<Style, ApiError> {
    let s = Style {
        id: Uuid::nil(),
        tenant_id,
        name: req.name,
        category: req.category,
        og_min: req.og_min,
        og_max: req.og_max,
        fg_min: req.fg_min,
        fg_max: req.fg_max,
        abv_min: req.abv_min,
        abv_max: req.abv_max,
        ibu_min: req.ibu_min,
        ibu_max: req.ibu_max,
        color_ebc_min: req.color_ebc_min,
        color_ebc_max: req.color_ebc_max,
        description: req.description,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    Ok(repo::insert_style(pool, &s).await?)
}

/// Lists styles visible to the caller.
pub async fn list_styles(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: StyleFilter,
) -> Result<Page<Style>, ApiError> {
    repo::select_styles(pool, tenant_id, &filter).await
}

/// Fetches one style visible to the caller.
pub async fn get_style(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<Style, ApiError> {
    repo::select_style_by_id(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("style"))
}

/// Replaces a caller-owned style.
pub async fn replace_style(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: StyleRequest,
) -> Result<Style, ApiError> {
    let mut s = repo::select_owned_style(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("style"))?;
    s.name = req.name;
    s.category = req.category;
    s.og_min = req.og_min;
    s.og_max = req.og_max;
    s.fg_min = req.fg_min;
    s.fg_max = req.fg_max;
    s.abv_min = req.abv_min;
    s.abv_max = req.abv_max;
    s.ibu_min = req.ibu_min;
    s.ibu_max = req.ibu_max;
    s.color_ebc_min = req.color_ebc_min;
    s.color_ebc_max = req.color_ebc_max;
    s.description = req.description;
    repo::update_style(pool, &s)
        .await?
        .ok_or_else(|| ApiError::not_found("style"))
}

/// Partially updates a caller-owned style.
pub async fn patch_style(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchStyleRequest,
) -> Result<Style, ApiError> {
    let mut s = repo::select_owned_style(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("style"))?;
    if let Some(v) = req.name {
        s.name = v;
    }
    if req.category.is_some() {
        s.category = req.category;
    }
    if req.og_min.is_some() {
        s.og_min = req.og_min;
    }
    if req.og_max.is_some() {
        s.og_max = req.og_max;
    }
    if req.fg_min.is_some() {
        s.fg_min = req.fg_min;
    }
    if req.fg_max.is_some() {
        s.fg_max = req.fg_max;
    }
    if req.abv_min.is_some() {
        s.abv_min = req.abv_min;
    }
    if req.abv_max.is_some() {
        s.abv_max = req.abv_max;
    }
    if req.ibu_min.is_some() {
        s.ibu_min = req.ibu_min;
    }
    if req.ibu_max.is_some() {
        s.ibu_max = req.ibu_max;
    }
    if req.color_ebc_min.is_some() {
        s.color_ebc_min = req.color_ebc_min;
    }
    if req.color_ebc_max.is_some() {
        s.color_ebc_max = req.color_ebc_max;
    }
    if req.description.is_some() {
        s.description = req.description;
    }
    repo::update_style(pool, &s)
        .await?
        .ok_or_else(|| ApiError::not_found("style"))
}

/// Deletes a caller-owned style.
pub async fn delete_style(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::delete_style(pool, tenant_id, id).await? == 0 {
        return Err(ApiError::not_found("style"));
    }
    Ok(())
}

// ---- Equipment profiles ----

/// Creates an equipment profile owned by the caller.
pub async fn create_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    req: EquipmentRequest,
) -> Result<EquipmentProfile, ApiError> {
    let ep = EquipmentProfile {
        id: Uuid::nil(),
        tenant_id,
        name: req.name,
        batch_size_liters: req.batch_size_liters,
        batch_volume_target_liters: req.batch_volume_target_liters,
        element_power_watts: req.element_power_watts,
        boil_size_liters: req.boil_size_liters,
        pre_boil_volume_liters: req.pre_boil_volume_liters,
        boil_time_minutes: req.boil_time_minutes,
        boil_off_rate_liters_per_hour: req.boil_off_rate_liters_per_hour,
        boil_temp_c: req.boil_temp_c,
        trub_loss_liters: req.trub_loss_liters,
        mash_tun_deadspace_liters: req.mash_tun_deadspace_liters,
        mash_tun_loss_liters: req.mash_tun_loss_liters,
        hlt_deadspace_liters: req.hlt_deadspace_liters,
        fermenter_loss_liters: req.fermenter_loss_liters,
        top_up_liters: req.top_up_liters,
        mash_time_minutes: req.mash_time_minutes,
        brewhouse_efficiency_pct: req.brewhouse_efficiency_pct,
        mash_efficiency_pct: req.mash_efficiency_pct,
        hop_utilisation_pct: req.hop_utilisation_pct,
        aroma_hop_utilisation_pct: req.aroma_hop_utilisation_pct,
        hop_stand_temp_c: req.hop_stand_temp_c,
        altitude_m: req.altitude_m,
        cooling_shrinkage_pct: req.cooling_shrinkage_pct,
        grain_absorption_l_per_kg: req.grain_absorption_l_per_kg,
        water_to_grain_ratio: req.water_to_grain_ratio,
        sparge_water_reminder_liters: req.sparge_water_reminder_liters,
        notes: req.notes,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    Ok(repo::insert_equipment(pool, &ep).await?)
}

/// Lists equipment profiles visible to the caller.
pub async fn list_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: EquipmentFilter,
) -> Result<Page<EquipmentProfile>, ApiError> {
    repo::select_equipment(pool, tenant_id, &filter).await
}

/// Fetches one equipment profile visible to the caller.
pub async fn get_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<EquipmentProfile, ApiError> {
    repo::select_equipment_by_id(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("equipment profile"))
}

/// Replaces a caller-owned equipment profile.
pub async fn replace_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: EquipmentRequest,
) -> Result<EquipmentProfile, ApiError> {
    let mut ep = repo::select_owned_equipment(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("equipment profile"))?;
    ep.name = req.name;
    ep.batch_size_liters = req.batch_size_liters;
    ep.batch_volume_target_liters = req.batch_volume_target_liters;
    ep.element_power_watts = req.element_power_watts;
    ep.boil_size_liters = req.boil_size_liters;
    ep.pre_boil_volume_liters = req.pre_boil_volume_liters;
    ep.boil_time_minutes = req.boil_time_minutes;
    ep.boil_off_rate_liters_per_hour = req.boil_off_rate_liters_per_hour;
    ep.boil_temp_c = req.boil_temp_c;
    ep.trub_loss_liters = req.trub_loss_liters;
    ep.mash_tun_deadspace_liters = req.mash_tun_deadspace_liters;
    ep.mash_tun_loss_liters = req.mash_tun_loss_liters;
    ep.hlt_deadspace_liters = req.hlt_deadspace_liters;
    ep.fermenter_loss_liters = req.fermenter_loss_liters;
    ep.top_up_liters = req.top_up_liters;
    ep.mash_time_minutes = req.mash_time_minutes;
    ep.brewhouse_efficiency_pct = req.brewhouse_efficiency_pct;
    ep.mash_efficiency_pct = req.mash_efficiency_pct;
    ep.hop_utilisation_pct = req.hop_utilisation_pct;
    ep.aroma_hop_utilisation_pct = req.aroma_hop_utilisation_pct;
    ep.hop_stand_temp_c = req.hop_stand_temp_c;
    ep.altitude_m = req.altitude_m;
    ep.cooling_shrinkage_pct = req.cooling_shrinkage_pct;
    ep.grain_absorption_l_per_kg = req.grain_absorption_l_per_kg;
    ep.water_to_grain_ratio = req.water_to_grain_ratio;
    ep.sparge_water_reminder_liters = req.sparge_water_reminder_liters;
    ep.notes = req.notes;
    repo::update_equipment(pool, &ep)
        .await?
        .ok_or_else(|| ApiError::not_found("equipment profile"))
}

/// Partially updates a caller-owned equipment profile.
pub async fn patch_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchEquipmentRequest,
) -> Result<EquipmentProfile, ApiError> {
    let mut ep = repo::select_owned_equipment(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("equipment profile"))?;
    if let Some(v) = req.name {
        ep.name = v;
    }
    if req.batch_size_liters.is_some() {
        ep.batch_size_liters = req.batch_size_liters;
    }
    if req.batch_volume_target_liters.is_some() {
        ep.batch_volume_target_liters = req.batch_volume_target_liters;
    }
    if req.element_power_watts.is_some() {
        ep.element_power_watts = req.element_power_watts;
    }
    if req.boil_size_liters.is_some() {
        ep.boil_size_liters = req.boil_size_liters;
    }
    if req.pre_boil_volume_liters.is_some() {
        ep.pre_boil_volume_liters = req.pre_boil_volume_liters;
    }
    if req.boil_time_minutes.is_some() {
        ep.boil_time_minutes = req.boil_time_minutes;
    }
    if req.boil_off_rate_liters_per_hour.is_some() {
        ep.boil_off_rate_liters_per_hour = req.boil_off_rate_liters_per_hour;
    }
    if req.boil_temp_c.is_some() {
        ep.boil_temp_c = req.boil_temp_c;
    }
    if req.trub_loss_liters.is_some() {
        ep.trub_loss_liters = req.trub_loss_liters;
    }
    if req.mash_tun_deadspace_liters.is_some() {
        ep.mash_tun_deadspace_liters = req.mash_tun_deadspace_liters;
    }
    if req.mash_tun_loss_liters.is_some() {
        ep.mash_tun_loss_liters = req.mash_tun_loss_liters;
    }
    if req.hlt_deadspace_liters.is_some() {
        ep.hlt_deadspace_liters = req.hlt_deadspace_liters;
    }
    if req.fermenter_loss_liters.is_some() {
        ep.fermenter_loss_liters = req.fermenter_loss_liters;
    }
    if req.top_up_liters.is_some() {
        ep.top_up_liters = req.top_up_liters;
    }
    if req.mash_time_minutes.is_some() {
        ep.mash_time_minutes = req.mash_time_minutes;
    }
    if req.brewhouse_efficiency_pct.is_some() {
        ep.brewhouse_efficiency_pct = req.brewhouse_efficiency_pct;
    }
    if req.mash_efficiency_pct.is_some() {
        ep.mash_efficiency_pct = req.mash_efficiency_pct;
    }
    if req.hop_utilisation_pct.is_some() {
        ep.hop_utilisation_pct = req.hop_utilisation_pct;
    }
    if req.aroma_hop_utilisation_pct.is_some() {
        ep.aroma_hop_utilisation_pct = req.aroma_hop_utilisation_pct;
    }
    if req.hop_stand_temp_c.is_some() {
        ep.hop_stand_temp_c = req.hop_stand_temp_c;
    }
    if req.altitude_m.is_some() {
        ep.altitude_m = req.altitude_m;
    }
    if req.cooling_shrinkage_pct.is_some() {
        ep.cooling_shrinkage_pct = req.cooling_shrinkage_pct;
    }
    if req.grain_absorption_l_per_kg.is_some() {
        ep.grain_absorption_l_per_kg = req.grain_absorption_l_per_kg;
    }
    if req.water_to_grain_ratio.is_some() {
        ep.water_to_grain_ratio = req.water_to_grain_ratio;
    }
    if req.sparge_water_reminder_liters.is_some() {
        ep.sparge_water_reminder_liters = req.sparge_water_reminder_liters;
    }
    if req.notes.is_some() {
        ep.notes = req.notes;
    }
    repo::update_equipment(pool, &ep)
        .await?
        .ok_or_else(|| ApiError::not_found("equipment profile"))
}

/// Deletes a caller-owned equipment profile.
pub async fn delete_equipment(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::delete_equipment(pool, tenant_id, id).await? == 0 {
        return Err(ApiError::not_found("equipment profile"));
    }
    Ok(())
}

// ---- Mash profiles ----

/// Creates a mash profile (with steps) owned by the caller.
pub async fn create_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    req: MashProfileRequest,
) -> Result<MashProfile, ApiError> {
    Ok(repo::insert_mash_profile(
        pool,
        tenant_id,
        &req.name,
        req.notes.as_deref(),
        &req.mash_steps,
    )
    .await?)
}

/// Lists mash profiles visible to the caller.
pub async fn list_mash_profiles(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: MashFilter,
) -> Result<Page<MashProfile>, ApiError> {
    repo::select_mash_profiles(pool, tenant_id, &filter).await
}

/// Fetches one mash profile visible to the caller.
pub async fn get_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<MashProfile, ApiError> {
    repo::select_mash_profile_by_id(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("mash profile"))
}

/// Replaces a caller-owned mash profile and its steps.
pub async fn replace_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: MashProfileRequest,
) -> Result<MashProfile, ApiError> {
    repo::update_mash_profile(
        pool,
        tenant_id,
        id,
        &req.name,
        req.notes.as_deref(),
        &req.mash_steps,
    )
    .await?
    .ok_or_else(|| ApiError::not_found("mash profile"))
}

/// Partially updates a caller-owned mash profile (and optionally its steps).
pub async fn patch_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchMashProfileRequest,
) -> Result<MashProfile, ApiError> {
    let existing = repo::select_owned_mash_profile(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("mash profile"))?;

    let name = req.name.unwrap_or(existing.name);
    let notes = if req.notes.is_some() {
        req.notes
    } else {
        existing.notes
    };
    // Steps are only replaced when present in the payload; otherwise keep current.
    let steps: Vec<MashStepRequest> = match req.mash_steps {
        Some(steps) => steps,
        None => repo::load_mash_steps(pool, id)
            .await?
            .into_iter()
            .map(|s| MashStepRequest {
                step_order: s.step_order,
                step_type: s.step_type,
                target_temp_c: s.target_temp_c,
                hold_minutes: s.hold_minutes,
                infusion_volume_liters: s.infusion_volume_liters,
            })
            .collect(),
    };

    repo::update_mash_profile(pool, tenant_id, id, &name, notes.as_deref(), &steps)
        .await?
        .ok_or_else(|| ApiError::not_found("mash profile"))
}

/// Deletes a caller-owned mash profile.
pub async fn delete_mash_profile(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::delete_mash_profile(pool, tenant_id, id).await? == 0 {
        return Err(ApiError::not_found("mash profile"));
    }
    Ok(())
}

// ---- Yeasts ----

/// Creates a yeast owned by the caller.
pub async fn create_yeast(
    pool: &PgPool,
    tenant_id: Uuid,
    req: YeastRequest,
) -> Result<Yeast, ApiError> {
    let y = Yeast {
        id: Uuid::nil(),
        tenant_id,
        name: req.name,
        manufacturer: req.manufacturer,
        product_code: req.product_code,
        yeast_type: req.yeast_type,
        form: req.form,
        attenuation_min_pct: req.attenuation_min_pct,
        attenuation_max_pct: req.attenuation_max_pct,
        temp_min_c: req.temp_min_c,
        temp_max_c: req.temp_max_c,
        flocculation: req.flocculation,
        notes: req.notes,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    Ok(repo::insert_yeast(pool, &y).await?)
}

/// Lists yeasts visible to the caller.
pub async fn list_yeasts(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: YeastFilter,
) -> Result<Page<Yeast>, ApiError> {
    repo::select_yeasts(pool, tenant_id, &filter).await
}

/// Fetches one yeast visible to the caller.
pub async fn get_yeast(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<Yeast, ApiError> {
    repo::select_yeast_by_id(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast"))
}

/// Replaces a caller-owned yeast.
pub async fn replace_yeast(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: YeastRequest,
) -> Result<Yeast, ApiError> {
    let mut y = repo::select_owned_yeast(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast"))?;
    y.name = req.name;
    y.manufacturer = req.manufacturer;
    y.product_code = req.product_code;
    y.yeast_type = req.yeast_type;
    y.form = req.form;
    y.attenuation_min_pct = req.attenuation_min_pct;
    y.attenuation_max_pct = req.attenuation_max_pct;
    y.temp_min_c = req.temp_min_c;
    y.temp_max_c = req.temp_max_c;
    y.flocculation = req.flocculation;
    y.notes = req.notes;
    repo::update_yeast(pool, &y)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast"))
}

/// Partially updates a caller-owned yeast.
pub async fn patch_yeast(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchYeastRequest,
) -> Result<Yeast, ApiError> {
    let mut y = repo::select_owned_yeast(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast"))?;
    if let Some(v) = req.name {
        y.name = v;
    }
    if req.manufacturer.is_some() {
        y.manufacturer = req.manufacturer;
    }
    if req.product_code.is_some() {
        y.product_code = req.product_code;
    }
    if req.yeast_type.is_some() {
        y.yeast_type = req.yeast_type;
    }
    if req.form.is_some() {
        y.form = req.form;
    }
    if req.attenuation_min_pct.is_some() {
        y.attenuation_min_pct = req.attenuation_min_pct;
    }
    if req.attenuation_max_pct.is_some() {
        y.attenuation_max_pct = req.attenuation_max_pct;
    }
    if req.temp_min_c.is_some() {
        y.temp_min_c = req.temp_min_c;
    }
    if req.temp_max_c.is_some() {
        y.temp_max_c = req.temp_max_c;
    }
    if req.flocculation.is_some() {
        y.flocculation = req.flocculation;
    }
    if req.notes.is_some() {
        y.notes = req.notes;
    }
    repo::update_yeast(pool, &y)
        .await?
        .ok_or_else(|| ApiError::not_found("yeast"))
}

/// Deletes a caller-owned yeast.
pub async fn delete_yeast(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::delete_yeast(pool, tenant_id, id).await? == 0 {
        return Err(ApiError::not_found("yeast"));
    }
    Ok(())
}

// ---- Library fermentables ----

/// Creates a library fermentable owned by the caller.
pub async fn create_fermentable(
    pool: &PgPool,
    tenant_id: Uuid,
    req: FermentableRequest,
) -> Result<Fermentable, ApiError> {
    let f = Fermentable {
        id: Uuid::nil(),
        tenant_id,
        name: req.name,
        supplier: req.supplier,
        fermentable_type: req.fermentable_type,
        colour_ebc_min: req.colour_ebc_min,
        colour_ebc_max: req.colour_ebc_max,
        extract_litres_per_kg: req.extract_litres_per_kg,
        moisture_pct_max: req.moisture_pct_max,
        tn_min: req.tn_min,
        tn_max: req.tn_max,
        snr_min: req.snr_min,
        snr_max: req.snr_max,
        attributes: req.attributes,
        notes: req.notes,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    Ok(repo::insert_fermentable(pool, &f).await?)
}

/// Lists library fermentables visible to the caller.
pub async fn list_fermentables(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: FermentableFilter,
) -> Result<Page<Fermentable>, ApiError> {
    repo::select_fermentables(pool, tenant_id, &filter).await
}

/// Fetches one library fermentable visible to the caller.
pub async fn get_fermentable(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Fermentable, ApiError> {
    repo::select_fermentable_by_id(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("library fermentable"))
}

/// Replaces a caller-owned library fermentable.
pub async fn replace_fermentable(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: FermentableRequest,
) -> Result<Fermentable, ApiError> {
    let mut f = repo::select_owned_fermentable(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("library fermentable"))?;
    f.name = req.name;
    f.supplier = req.supplier;
    f.fermentable_type = req.fermentable_type;
    f.colour_ebc_min = req.colour_ebc_min;
    f.colour_ebc_max = req.colour_ebc_max;
    f.extract_litres_per_kg = req.extract_litres_per_kg;
    f.moisture_pct_max = req.moisture_pct_max;
    f.tn_min = req.tn_min;
    f.tn_max = req.tn_max;
    f.snr_min = req.snr_min;
    f.snr_max = req.snr_max;
    f.attributes = req.attributes;
    f.notes = req.notes;
    repo::update_fermentable(pool, &f)
        .await?
        .ok_or_else(|| ApiError::not_found("library fermentable"))
}

/// Partially updates a caller-owned library fermentable.
pub async fn patch_fermentable(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchFermentableRequest,
) -> Result<Fermentable, ApiError> {
    let mut f = repo::select_owned_fermentable(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("library fermentable"))?;
    if let Some(v) = req.name {
        f.name = v;
    }
    if req.supplier.is_some() {
        f.supplier = req.supplier;
    }
    if req.fermentable_type.is_some() {
        f.fermentable_type = req.fermentable_type;
    }
    if req.colour_ebc_min.is_some() {
        f.colour_ebc_min = req.colour_ebc_min;
    }
    if req.colour_ebc_max.is_some() {
        f.colour_ebc_max = req.colour_ebc_max;
    }
    if req.extract_litres_per_kg.is_some() {
        f.extract_litres_per_kg = req.extract_litres_per_kg;
    }
    if req.moisture_pct_max.is_some() {
        f.moisture_pct_max = req.moisture_pct_max;
    }
    if req.tn_min.is_some() {
        f.tn_min = req.tn_min;
    }
    if req.tn_max.is_some() {
        f.tn_max = req.tn_max;
    }
    if req.snr_min.is_some() {
        f.snr_min = req.snr_min;
    }
    if req.snr_max.is_some() {
        f.snr_max = req.snr_max;
    }
    if req.attributes.is_some() {
        f.attributes = req.attributes;
    }
    if req.notes.is_some() {
        f.notes = req.notes;
    }
    repo::update_fermentable(pool, &f)
        .await?
        .ok_or_else(|| ApiError::not_found("library fermentable"))
}

/// Deletes a caller-owned library fermentable.
pub async fn delete_fermentable(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::delete_fermentable(pool, tenant_id, id).await? == 0 {
        return Err(ApiError::not_found("library fermentable"));
    }
    Ok(())
}
