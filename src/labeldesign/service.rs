//! Label-design business logic: brand assets, brand profiles, designs, and
//! render-model resolution.
//!
//! Port of the Go `internal/labeldesign/service.go`. Brand assets are bounded to
//! 2 MiB and limited to PNG/JPEG. Designs are batch-bound (bottle/can) or
//! recipe-bound (pump_clip/cask_lens); the render model resolves brand and
//! compliance/recipe fields from the respective modules.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use uuid::Uuid;

use super::models::{
    BrandAsset, BrandProfile, CreateBrandProfileRequest, CreateLabelDesignRequest, LabelDesign,
    ListFilter, Page, PatchBrandProfileRequest, PatchLabelDesignRequest,
};
use super::repository as repo;
use crate::pkg::labelkit::{self, RenderBrand, RenderFields, RenderModel, RenderTasting};
use crate::platform::errors::ApiError;
use crate::state::AppState;

const MAX_ASSET_BYTES: usize = 2 * 1024 * 1024; // 2 MiB

/// True if the error is a unique-constraint violation.
fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

/// `""` → `None`, matching the Go `strPtr` helper.
fn str_ptr(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

// ---- brand assets ----

pub async fn upload_asset(
    state: &AppState,
    tenant_id: Uuid,
    filename: &str,
    content_type: &str,
    data: &[u8],
) -> Result<BrandAsset, ApiError> {
    if content_type != "image/png" && content_type != "image/jpeg" {
        return Err(ApiError::unsupported_media_type());
    }
    if data.is_empty() {
        return Err(ApiError::validation("file", "must not be empty"));
    }
    if data.len() > MAX_ASSET_BYTES {
        return Err(ApiError::validation("file", "must not exceed 2 MiB"));
    }
    Ok(repo::insert_asset(
        &state.pool,
        tenant_id,
        filename,
        content_type,
        data.len() as i32,
        data,
    )
    .await?)
}

pub async fn get_asset_data(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<(BrandAsset, Vec<u8>), ApiError> {
    repo::select_asset(&state.pool, tenant_id, id).await
}

pub async fn delete_asset(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    repo::delete_asset(&state.pool, tenant_id, id).await
}

// ---- brand profiles ----

/// Verifies an optional logo asset exists; `validation` error if it does not.
async fn verify_asset(
    state: &AppState,
    tenant_id: Uuid,
    asset_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let Some(id) = asset_id else {
        return Ok(());
    };
    match repo::select_asset(&state.pool, tenant_id, id).await {
        Ok(_) => Ok(()),
        Err(e) if e.code() == "not_found" => Err(ApiError::validation(
            "logo_asset_id",
            "brand asset does not exist",
        )),
        Err(e) => Err(e),
    }
}

pub async fn create_profile(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateBrandProfileRequest,
) -> Result<BrandProfile, ApiError> {
    let primary = req.primary_color.unwrap_or_else(|| "#000000".to_string());
    let secondary = req.secondary_color.unwrap_or_else(|| "#ffffff".to_string());
    let font = req.font_family.unwrap_or_else(|| "helvetica".to_string());

    verify_asset(state, tenant_id, req.logo_asset_id).await?;

    match repo::insert_profile(
        &state.pool,
        tenant_id,
        &req.name,
        &primary,
        &secondary,
        &font,
        req.logo_asset_id,
    )
    .await
    {
        Ok(p) => Ok(p),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "brand_profile",
            "a brand profile with this name already exists",
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn list_profiles(
    state: &AppState,
    tenant_id: Uuid,
) -> Result<Vec<BrandProfile>, ApiError> {
    Ok(repo::select_profiles(&state.pool, tenant_id).await?)
}

pub async fn get_profile(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<BrandProfile, ApiError> {
    repo::select_profile_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("brand_profile"))
}

pub async fn patch_profile(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchBrandProfileRequest,
) -> Result<BrandProfile, ApiError> {
    let mut p = get_profile(state, tenant_id, id).await?;
    if let Some(v) = req.name {
        p.name = v;
    }
    if let Some(v) = req.primary_color {
        p.primary_color = v;
    }
    if let Some(v) = req.secondary_color {
        p.secondary_color = v;
    }
    if let Some(v) = req.font_family {
        p.font_family = v;
    }
    if req.logo_asset_id.is_some() {
        verify_asset(state, tenant_id, req.logo_asset_id).await?;
        p.logo_asset_id = req.logo_asset_id;
    }
    match repo::update_profile(
        &state.pool,
        tenant_id,
        id,
        &p.name,
        &p.primary_color,
        &p.secondary_color,
        &p.font_family,
        p.logo_asset_id,
    )
    .await
    {
        Ok(Some(updated)) => Ok(updated),
        Ok(None) => Err(ApiError::not_found("brand_profile")),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "brand_profile",
            "a brand profile with this name already exists",
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn delete_profile(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if !repo::delete_profile(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("brand_profile"));
    }
    Ok(())
}

// ---- designs ----

pub async fn create_design(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateLabelDesignRequest,
) -> Result<LabelDesign, ApiError> {
    validate_kind_source(&req.kind, req.batch_id, req.recipe_id)?;
    validate_size_template(&req.kind, &req.size_key, &req.template_key)?;

    if let Some(profile_id) = req.brand_profile_id {
        if repo::select_profile_by_id(&state.pool, tenant_id, profile_id)
            .await?
            .is_none()
        {
            return Err(ApiError::validation(
                "brand_profile_id",
                "brand profile does not exist",
            ));
        }
    }

    let options = req.options.unwrap_or_default();
    Ok(repo::insert_design(
        &state.pool,
        tenant_id,
        &req.kind,
        &req.name,
        req.batch_id,
        req.recipe_id,
        req.brand_profile_id,
        &req.size_key,
        &req.template_key,
        &options,
    )
    .await?)
}

pub async fn list_designs(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<Page<LabelDesign>, ApiError> {
    Ok(repo::select_designs(&state.pool, tenant_id, &filter).await?)
}

pub async fn get_design(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<LabelDesign, ApiError> {
    repo::select_design_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("label_design"))
}

pub async fn patch_design(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchLabelDesignRequest,
) -> Result<LabelDesign, ApiError> {
    let mut d = get_design(state, tenant_id, id).await?;
    if let Some(v) = req.name {
        d.name = v;
    }
    if let Some(v) = req.size_key {
        d.size_key = v;
    }
    if let Some(v) = req.template_key {
        d.template_key = v;
    }
    validate_size_template(&d.kind, &d.size_key, &d.template_key)?;

    if req.brand_profile_id.is_some() {
        if let Some(profile_id) = req.brand_profile_id {
            if repo::select_profile_by_id(&state.pool, tenant_id, profile_id)
                .await?
                .is_none()
            {
                return Err(ApiError::validation(
                    "brand_profile_id",
                    "brand profile does not exist",
                ));
            }
        }
        d.brand_profile_id = req.brand_profile_id;
    }
    if let Some(v) = req.options {
        d.options = v;
    }

    repo::update_design(
        &state.pool,
        tenant_id,
        id,
        &d.name,
        d.brand_profile_id,
        &d.size_key,
        &d.template_key,
        &d.options,
    )
    .await?
    .ok_or_else(|| ApiError::not_found("label_design"))
}

pub async fn delete_design(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if !repo::delete_design(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("label_design"));
    }
    Ok(())
}

// ---- rendering ----

pub async fn render(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<RenderModel, ApiError> {
    let d = get_design(state, tenant_id, id).await?;

    let size = labelkit::size_preset(&d.size_key)
        .ok_or_else(|| ApiError::validation("size_key", "unknown size preset"))?;

    let brand = resolve_brand(state, tenant_id, &d).await?;
    let fields = resolve_fields(state, tenant_id, &d).await?;

    Ok(RenderModel {
        design_id: d.id,
        kind: d.kind.clone(),
        size_key: d.size_key.clone(),
        template_key: d.template_key.clone(),
        width_mm: size.width_mm,
        height_mm: size.height_mm,
        shape: size.shape.to_string(),
        brand,
        fields,
        options: d.options,
    })
}

pub async fn render_pdf(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Vec<u8>, ApiError> {
    let model = render(state, tenant_id, id).await?;
    let mut logo: Vec<u8> = Vec::new();
    if let Some(asset_id) = model.brand.logo_asset_id {
        if let Ok((_, data)) = repo::select_asset(&state.pool, tenant_id, asset_id).await {
            logo = data;
        }
    }
    labelkit::render_pdf(&model, &logo).map_err(ApiError::internal)
}

async fn resolve_brand(
    state: &AppState,
    tenant_id: Uuid,
    d: &LabelDesign,
) -> Result<RenderBrand, ApiError> {
    let mut brand = RenderBrand {
        brewery_name: String::new(),
        primary_color: "#000000".to_string(),
        secondary_color: "#ffffff".to_string(),
        font_family: "helvetica".to_string(),
        logo_asset_id: None,
    };

    // Load tenant for the brewery name; an error propagates (matches Go).
    match crate::tenant::repository::get_by_id(&state.pool, tenant_id).await {
        Ok(Some(tn)) => brand.brewery_name = tn.tenant_name,
        Ok(None) => return Err(ApiError::not_found("tenant")),
        Err(e) => return Err(e.into()),
    }

    if let Some(profile_id) = d.brand_profile_id {
        match repo::select_profile_by_id(&state.pool, tenant_id, profile_id).await? {
            // Profile deleted since; fall back to defaults.
            None => return Ok(brand),
            Some(p) => {
                brand.primary_color = p.primary_color;
                brand.secondary_color = p.secondary_color;
                brand.font_family = p.font_family;
                brand.logo_asset_id = p.logo_asset_id;
            }
        }
    }
    Ok(brand)
}

async fn resolve_fields(
    state: &AppState,
    tenant_id: Uuid,
    d: &LabelDesign,
) -> Result<RenderFields, ApiError> {
    match d.kind.as_str() {
        "bottle" | "can" => {
            let batch_id = d
                .batch_id
                .ok_or_else(|| ApiError::internal("bottle/can design missing batch_id"))?;
            resolve_compliance_fields(state, tenant_id, batch_id).await
        }
        _ => {
            let recipe_id = d
                .recipe_id
                .ok_or_else(|| ApiError::internal("clip/lens design missing recipe_id"))?;
            resolve_recipe_fields(state, tenant_id, recipe_id).await
        }
    }
}

async fn resolve_compliance_fields(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<RenderFields, ApiError> {
    let rec = match crate::labels::service::latest_approved_for_batch(state, tenant_id, batch_id)
        .await
    {
        Ok(rec) => rec,
        Err(e) if e.code() == "not_found" => {
            return Err(ApiError::business_rule(
                "no_approved_label_record",
                "This batch has no approved label record. Approve a compliance label before printing.",
                BTreeMap::new(),
            ));
        }
        Err(e) => return Err(e),
    };

    let best_before_date = rec.best_before_date.as_deref().and_then(|s| {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(labelkit::format_best_before)
    });

    Ok(RenderFields {
        product_name: rec.product_name,
        style: None,
        abv_percent: rec.abv_percent,
        allergens: rec.allergens,
        net_volume_ml: Some(rec.net_volume_ml),
        responsible_party: str_ptr(rec.responsible_party),
        country_of_origin: str_ptr(rec.country_of_origin),
        best_before_date,
        lot_identifier: str_ptr(rec.lot_identifier),
        ingredient_list: rec.ingredient_list,
        energy_kj_per_100ml: rec.energy_kj_per_100ml,
        energy_kcal_per_100ml: rec.energy_kcal_per_100ml,
        alcohol_units_per_serving: rec.alcohol_units_per_serving,
        tasting: None,
    })
}

async fn resolve_recipe_fields(
    state: &AppState,
    tenant_id: Uuid,
    recipe_id: Uuid,
) -> Result<RenderFields, ApiError> {
    let rw = crate::recipe::service::get(state, tenant_id, recipe_id).await?;

    let mut fields = RenderFields {
        product_name: rw.recipe.name.clone(),
        abv_percent: rw.recipe.calc_abv_pct.unwrap_or(0.0),
        ..RenderFields::default()
    };

    if let Some(style_id) = rw.recipe.style_id {
        if let Ok(style) =
            crate::library::service::get_style(&state.pool, tenant_id, style_id).await
        {
            fields.style = Some(style.name);
        }
    }

    if rw.recipe.tasting_aroma.is_some()
        || rw.recipe.tasting_flavour.is_some()
        || rw.recipe.tasting_mouthfeel.is_some()
        || rw.recipe.tasting_finish.is_some()
    {
        fields.tasting = Some(RenderTasting {
            aroma: rw.recipe.tasting_aroma.clone(),
            flavour: rw.recipe.tasting_flavour.clone(),
            mouthfeel: rw.recipe.tasting_mouthfeel.clone(),
            finish: rw.recipe.tasting_finish.clone(),
        });
    }

    Ok(fields)
}

// ---- validation helpers ----

fn validate_kind_source(
    kind: &str,
    batch_id: Option<Uuid>,
    recipe_id: Option<Uuid>,
) -> Result<(), ApiError> {
    match kind {
        "bottle" | "can" => {
            if batch_id.is_none() || recipe_id.is_some() {
                return Err(ApiError::business_rule(
                    "kind_source_mismatch",
                    "bottle and can designs require a batch_id and no recipe_id",
                    BTreeMap::new(),
                ));
            }
        }
        "pump_clip" | "cask_lens" => {
            if recipe_id.is_none() || batch_id.is_some() {
                return Err(ApiError::business_rule(
                    "kind_source_mismatch",
                    "pump_clip and cask_lens designs require a recipe_id and no batch_id",
                    BTreeMap::new(),
                ));
            }
        }
        _ => return Err(ApiError::validation("kind", "unknown kind")),
    }
    Ok(())
}

fn validate_size_template(kind: &str, size_key: &str, template_key: &str) -> Result<(), ApiError> {
    if labelkit::size_preset(size_key).is_none() {
        return Err(ApiError::validation("size_key", "unknown size preset"));
    }
    if !labelkit::valid_size_for_kind(size_key, kind) {
        return Err(ApiError::validation(
            "size_key",
            "size preset is not valid for this kind",
        ));
    }
    if labelkit::template_preset(template_key).is_none() {
        return Err(ApiError::validation(
            "template_key",
            "unknown template preset",
        ));
    }
    Ok(())
}

/// Sanitises a name for use as a PDF filename, matching the Go `sanitizeFilename`.
pub fn sanitize_filename(name: &str) -> String {
    if name.is_empty() {
        return "label".to_string();
    }
    let mapped: String = name
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => Some(c),
            ' ' => Some('_'),
            _ => None,
        })
        .collect();
    if mapped.is_empty() {
        "label".to_string()
    } else {
        mapped
    }
}
