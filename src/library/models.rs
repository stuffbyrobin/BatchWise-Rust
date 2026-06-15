//! Library domain types and DTOs.
//!
//! Port of the Go `internal/library` models (`library.go`). Covers beer styles,
//! equipment profiles, mash profiles (with nested steps), yeasts, and library
//! fermentables. `NUMERIC` columns are selected as `float8` so they decode into
//! `f64`; `INTEGER` columns map to `i32`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// The reserved tenant id for system-owned (shared) library data.
pub const SYSTEM_TENANT_ID: Uuid = Uuid::nil();

/// A generic paginated response envelope. Mirrors the Go `Page[T]`.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
    pub total_pages: i32,
}

impl<T> Page<T> {
    /// Builds a page envelope, computing the total page count.
    pub fn new(items: Vec<T>, total: i64, page: i32, page_size: i32) -> Self {
        let total_pages = if page_size > 0 && total > 0 {
            ((total + i64::from(page_size) - 1) / i64::from(page_size)) as i32
        } else {
            0
        };
        Page {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

// ---- Beer styles ----

/// A beer style record.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Style {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub og_min: Option<f64>,
    pub og_max: Option<f64>,
    pub fg_min: Option<f64>,
    pub fg_max: Option<f64>,
    pub abv_min: Option<f64>,
    pub abv_max: Option<f64>,
    pub ibu_min: Option<f64>,
    pub ibu_max: Option<f64>,
    pub color_ebc_min: Option<f64>,
    pub color_ebc_max: Option<f64>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/replace payload for a [`Style`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StyleRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub category: Option<String>,
    pub og_min: Option<f64>,
    pub og_max: Option<f64>,
    pub fg_min: Option<f64>,
    pub fg_max: Option<f64>,
    pub abv_min: Option<f64>,
    pub abv_max: Option<f64>,
    pub ibu_min: Option<f64>,
    pub ibu_max: Option<f64>,
    pub color_ebc_min: Option<f64>,
    pub color_ebc_max: Option<f64>,
    pub description: Option<String>,
}

/// Partial-update payload for a [`Style`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchStyleRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub og_min: Option<f64>,
    pub og_max: Option<f64>,
    pub fg_min: Option<f64>,
    pub fg_max: Option<f64>,
    pub abv_min: Option<f64>,
    pub abv_max: Option<f64>,
    pub ibu_min: Option<f64>,
    pub ibu_max: Option<f64>,
    pub color_ebc_min: Option<f64>,
    pub color_ebc_max: Option<f64>,
    pub description: Option<String>,
}

/// List-query parameters for styles.
#[derive(Debug, Default, Deserialize)]
pub struct StyleFilter {
    pub category: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub page_size: i32,
    #[serde(default)]
    pub sort: String,
}

// ---- Equipment profiles ----

/// A brewing equipment configuration.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct EquipmentProfile {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub batch_size_liters: Option<f64>,
    pub batch_volume_target_liters: Option<f64>,
    pub element_power_watts: Option<f64>,
    pub boil_size_liters: Option<f64>,
    pub pre_boil_volume_liters: Option<f64>,
    pub boil_time_minutes: Option<i32>,
    pub boil_off_rate_liters_per_hour: Option<f64>,
    pub boil_temp_c: Option<f64>,
    pub trub_loss_liters: Option<f64>,
    pub mash_tun_deadspace_liters: Option<f64>,
    pub mash_tun_loss_liters: Option<f64>,
    pub hlt_deadspace_liters: Option<f64>,
    pub fermenter_loss_liters: Option<f64>,
    pub top_up_liters: Option<f64>,
    pub mash_time_minutes: Option<i32>,
    pub brewhouse_efficiency_pct: Option<f64>,
    pub mash_efficiency_pct: Option<f64>,
    pub hop_utilisation_pct: Option<f64>,
    pub aroma_hop_utilisation_pct: Option<f64>,
    pub hop_stand_temp_c: Option<f64>,
    pub altitude_m: Option<f64>,
    pub cooling_shrinkage_pct: Option<f64>,
    pub grain_absorption_l_per_kg: Option<f64>,
    pub water_to_grain_ratio: Option<f64>,
    pub sparge_water_reminder_liters: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/replace payload for an [`EquipmentProfile`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct EquipmentRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub batch_size_liters: Option<f64>,
    pub batch_volume_target_liters: Option<f64>,
    pub element_power_watts: Option<f64>,
    pub boil_size_liters: Option<f64>,
    pub pre_boil_volume_liters: Option<f64>,
    pub boil_time_minutes: Option<i32>,
    pub boil_off_rate_liters_per_hour: Option<f64>,
    pub boil_temp_c: Option<f64>,
    pub trub_loss_liters: Option<f64>,
    pub mash_tun_deadspace_liters: Option<f64>,
    pub mash_tun_loss_liters: Option<f64>,
    pub hlt_deadspace_liters: Option<f64>,
    pub fermenter_loss_liters: Option<f64>,
    pub top_up_liters: Option<f64>,
    pub mash_time_minutes: Option<i32>,
    pub brewhouse_efficiency_pct: Option<f64>,
    pub mash_efficiency_pct: Option<f64>,
    pub hop_utilisation_pct: Option<f64>,
    pub aroma_hop_utilisation_pct: Option<f64>,
    pub hop_stand_temp_c: Option<f64>,
    pub altitude_m: Option<f64>,
    pub cooling_shrinkage_pct: Option<f64>,
    pub grain_absorption_l_per_kg: Option<f64>,
    pub water_to_grain_ratio: Option<f64>,
    pub sparge_water_reminder_liters: Option<f64>,
    pub notes: Option<String>,
}

/// Partial-update payload for an [`EquipmentProfile`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchEquipmentRequest {
    pub name: Option<String>,
    pub batch_size_liters: Option<f64>,
    pub batch_volume_target_liters: Option<f64>,
    pub element_power_watts: Option<f64>,
    pub boil_size_liters: Option<f64>,
    pub pre_boil_volume_liters: Option<f64>,
    pub boil_time_minutes: Option<i32>,
    pub boil_off_rate_liters_per_hour: Option<f64>,
    pub boil_temp_c: Option<f64>,
    pub trub_loss_liters: Option<f64>,
    pub mash_tun_deadspace_liters: Option<f64>,
    pub mash_tun_loss_liters: Option<f64>,
    pub hlt_deadspace_liters: Option<f64>,
    pub fermenter_loss_liters: Option<f64>,
    pub top_up_liters: Option<f64>,
    pub mash_time_minutes: Option<i32>,
    pub brewhouse_efficiency_pct: Option<f64>,
    pub mash_efficiency_pct: Option<f64>,
    pub hop_utilisation_pct: Option<f64>,
    pub aroma_hop_utilisation_pct: Option<f64>,
    pub hop_stand_temp_c: Option<f64>,
    pub altitude_m: Option<f64>,
    pub cooling_shrinkage_pct: Option<f64>,
    pub grain_absorption_l_per_kg: Option<f64>,
    pub water_to_grain_ratio: Option<f64>,
    pub sparge_water_reminder_liters: Option<f64>,
    pub notes: Option<String>,
}

/// List-query parameters for equipment profiles.
#[derive(Debug, Default, Deserialize)]
pub struct EquipmentFilter {
    pub name: Option<String>,
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub page_size: i32,
    #[serde(default)]
    pub sort: String,
}

// ---- Mash profiles ----

/// One step within a [`MashProfile`].
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MashStep {
    pub id: Uuid,
    pub mash_profile_id: Uuid,
    pub step_order: i32,
    pub step_type: String,
    pub target_temp_c: f64,
    pub hold_minutes: i32,
    pub infusion_volume_liters: Option<f64>,
}

/// A named mash schedule containing ordered steps.
#[derive(Debug, Clone, Serialize)]
pub struct MashProfile {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub notes: Option<String>,
    pub mash_steps: Vec<MashStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Header row for a mash profile, without its steps (joined separately).
#[derive(Debug, Clone, FromRow)]
pub struct MashProfileRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MashProfileRow {
    /// Combines the header row with its loaded steps into a full profile.
    pub fn with_steps(self, mash_steps: Vec<MashStep>) -> MashProfile {
        MashProfile {
            id: self.id,
            tenant_id: self.tenant_id,
            name: self.name,
            notes: self.notes,
            mash_steps,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Payload for a single mash step.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct MashStepRequest {
    #[validate(range(min = 1))]
    pub step_order: i32,
    #[validate(custom(function = "validate_step_type"))]
    pub step_type: String,
    pub target_temp_c: f64,
    #[validate(range(min = 1))]
    pub hold_minutes: i32,
    pub infusion_volume_liters: Option<f64>,
}

/// Validates the `step_type` enum (mirrors the Go `oneof` tag).
fn validate_step_type(value: &str) -> Result<(), validator::ValidationError> {
    match value {
        "infusion" | "temperature" | "decoction" => Ok(()),
        _ => Err(validator::ValidationError::new("step_type")),
    }
}

/// Create/replace payload for a [`MashProfile`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct MashProfileRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub notes: Option<String>,
    #[serde(default)]
    #[validate(nested)]
    pub mash_steps: Vec<MashStepRequest>,
}

/// Partial-update payload for a [`MashProfile`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchMashProfileRequest {
    pub name: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    #[validate(nested)]
    pub mash_steps: Option<Vec<MashStepRequest>>,
}

/// List-query parameters for mash profiles.
#[derive(Debug, Default, Deserialize)]
pub struct MashFilter {
    pub name: Option<String>,
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub page_size: i32,
    #[serde(default)]
    pub sort: String,
}

// ---- Yeasts ----

/// A yeast strain record.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Yeast {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub manufacturer: Option<String>,
    pub product_code: Option<String>,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub yeast_type: Option<String>,
    pub form: Option<String>,
    pub attenuation_min_pct: Option<f64>,
    pub attenuation_max_pct: Option<f64>,
    pub temp_min_c: Option<f64>,
    pub temp_max_c: Option<f64>,
    pub flocculation: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/replace payload for a [`Yeast`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct YeastRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub manufacturer: Option<String>,
    pub product_code: Option<String>,
    #[serde(rename = "type")]
    pub yeast_type: Option<String>,
    pub form: Option<String>,
    pub attenuation_min_pct: Option<f64>,
    pub attenuation_max_pct: Option<f64>,
    pub temp_min_c: Option<f64>,
    pub temp_max_c: Option<f64>,
    pub flocculation: Option<String>,
    pub notes: Option<String>,
}

/// Partial-update payload for a [`Yeast`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchYeastRequest {
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub product_code: Option<String>,
    #[serde(rename = "type")]
    pub yeast_type: Option<String>,
    pub form: Option<String>,
    pub attenuation_min_pct: Option<f64>,
    pub attenuation_max_pct: Option<f64>,
    pub temp_min_c: Option<f64>,
    pub temp_max_c: Option<f64>,
    pub flocculation: Option<String>,
    pub notes: Option<String>,
}

/// List-query parameters for yeasts.
#[derive(Debug, Default, Deserialize)]
pub struct YeastFilter {
    pub manufacturer: Option<String>,
    pub name: Option<String>,
    pub attenuation_min: Option<f64>,
    pub attenuation_max: Option<f64>,
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub page_size: i32,
    #[serde(default)]
    pub sort: String,
}

// ---- Library fermentables ----

/// A grain/adjunct reference record in the library.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Fermentable {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub supplier: Option<String>,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub fermentable_type: Option<String>,
    pub colour_ebc_min: Option<f64>,
    pub colour_ebc_max: Option<f64>,
    pub extract_litres_per_kg: Option<f64>,
    pub moisture_pct_max: Option<f64>,
    pub tn_min: Option<f64>,
    pub tn_max: Option<f64>,
    pub snr_min: Option<f64>,
    pub snr_max: Option<f64>,
    pub attributes: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/replace payload for a [`Fermentable`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FermentableRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub supplier: Option<String>,
    #[serde(rename = "type")]
    pub fermentable_type: Option<String>,
    pub colour_ebc_min: Option<f64>,
    pub colour_ebc_max: Option<f64>,
    pub extract_litres_per_kg: Option<f64>,
    pub moisture_pct_max: Option<f64>,
    pub tn_min: Option<f64>,
    pub tn_max: Option<f64>,
    pub snr_min: Option<f64>,
    pub snr_max: Option<f64>,
    pub attributes: Option<String>,
    pub notes: Option<String>,
}

/// Partial-update payload for a [`Fermentable`].
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchFermentableRequest {
    pub name: Option<String>,
    pub supplier: Option<String>,
    #[serde(rename = "type")]
    pub fermentable_type: Option<String>,
    pub colour_ebc_min: Option<f64>,
    pub colour_ebc_max: Option<f64>,
    pub extract_litres_per_kg: Option<f64>,
    pub moisture_pct_max: Option<f64>,
    pub tn_min: Option<f64>,
    pub tn_max: Option<f64>,
    pub snr_min: Option<f64>,
    pub snr_max: Option<f64>,
    pub attributes: Option<String>,
    pub notes: Option<String>,
}

/// List-query parameters for library fermentables.
#[derive(Debug, Default, Deserialize)]
pub struct FermentableFilter {
    pub name: Option<String>,
    pub supplier: Option<String>,
    #[serde(rename = "type")]
    pub fermentable_type: Option<String>,
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub page_size: i32,
    #[serde(default)]
    pub sort: String,
}
