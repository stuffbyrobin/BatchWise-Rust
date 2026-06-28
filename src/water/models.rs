//! Water chemistry domain types and DTOs.
//!
//! Port of the Go `internal/water` models (`water.go`). Covers named water
//! profiles and saved water-adjustment sessions (with cached calculation
//! results). `NUMERIC` columns are selected as `float8` so they decode into
//! `f64`; the additions arrays are stored as JSONB.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// The reserved tenant id for system-owned (shared) water profiles.
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

// ---- Water profiles ----

/// A named mineral composition stored in the database.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Profile {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub calcium_ppm: f64,
    pub magnesium_ppm: f64,
    pub sodium_ppm: f64,
    pub sulfate_ppm: f64,
    pub chloride_ppm: f64,
    pub bicarbonate_ppm: f64,
    pub notes: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Body for `POST /water-profiles` and `PUT /water-profiles/{id}`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateWaterProfileRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    #[validate(range(min = 0.0))]
    pub calcium_ppm: f64,
    #[validate(range(min = 0.0))]
    pub magnesium_ppm: f64,
    #[validate(range(min = 0.0))]
    pub sodium_ppm: f64,
    #[validate(range(min = 0.0))]
    pub sulfate_ppm: f64,
    #[validate(range(min = 0.0))]
    pub chloride_ppm: f64,
    #[validate(range(min = 0.0))]
    pub bicarbonate_ppm: f64,
    pub notes: Option<String>,
}

/// Body for `PUT /water-profiles/{id}`. All fields required (alias of create).
pub type UpdateWaterProfileRequest = CreateWaterProfileRequest;

/// Body for `PATCH /water-profiles/{id}`. All fields optional.
#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchWaterProfileRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub calcium_ppm: Option<f64>,
    pub magnesium_ppm: Option<f64>,
    pub sodium_ppm: Option<f64>,
    pub sulfate_ppm: Option<f64>,
    pub chloride_ppm: Option<f64>,
    pub bicarbonate_ppm: Option<f64>,
    pub notes: Option<String>,
}

// ---- Additions ----

/// One mineral salt addition in a water adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineralAddition {
    /// One of: "CaSO4", "CaCl2", "MgSO4", "MgCl2", "NaHCO3", "NaCl", "CaCO3",
    /// "Na2SO4", "CaOH2".
    pub r#type: String,
    /// Grams. For a liquid form this is the weight of solution.
    pub amount: f64,
    /// Salt form: "anhydrous", "dihydrate", or "liquid". Defaults to
    /// "dihydrate" when omitted. Currently only affects CaCl2.
    #[serde(default)]
    pub form: Option<String>,
    /// Solution strength (%w/w), used only when `form` is "liquid".
    #[serde(default)]
    pub strength_pct: Option<f64>,
}

/// One acid addition in a water adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcidAddition {
    /// One of: "lactic", "phosphoric", "sulfuric", "hydrochloric".
    pub r#type: String,
    /// Percent (e.g. 88.0 for 88% lactic).
    pub strength: f64,
    /// mL.
    pub amount: f64,
}

/// One grain in the mash for pH prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrainAddition {
    /// Informational name.
    pub name: String,
    /// One of: "base", "crystal", "roast", "acid".
    pub r#type: String,
    /// kg.
    pub weight: f64,
    /// °Lovibond.
    pub colour: f64,
}

/// Cached computed output of a water treatment calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    pub calcium_ppm: f64,
    pub magnesium_ppm: f64,
    pub sodium_ppm: f64,
    pub sulfate_ppm: f64,
    pub chloride_ppm: f64,
    pub bicarbonate_ppm: f64,
    /// ppm as CaCO3.
    pub alkalinity: f64,
    /// meq/L (Kolbach).
    pub residual_alk: f64,
    /// Ratio.
    pub sulfate_to_chloride: f64,
    /// 0.0 when no grain additions.
    pub mash_ph: f64,
}

// ---- Water adjustments ----

/// A saved water treatment session.
#[derive(Debug, Clone, Serialize)]
pub struct Adjustment {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub source_profile_id: Option<Uuid>,
    pub target_profile_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub volume_liters: f64,
    pub mineral_additions: Vec<MineralAddition>,
    pub acid_additions: Vec<AcidAddition>,
    pub grain_additions: Vec<GrainAddition>,
    pub result: Option<Result>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row scanned from `water_adjustments` before reshaping into [`Adjustment`].
#[derive(Debug, FromRow)]
pub struct AdjustmentRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub source_profile_id: Option<Uuid>,
    pub target_profile_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub volume_liters: f64,
    #[sqlx(json)]
    pub mineral_additions: Vec<MineralAddition>,
    #[sqlx(json)]
    pub acid_additions: Vec<AcidAddition>,
    #[sqlx(json)]
    pub grain_additions: Vec<GrainAddition>,
    pub result_calcium_ppm: Option<f64>,
    pub result_magnesium_ppm: Option<f64>,
    pub result_sodium_ppm: Option<f64>,
    pub result_sulfate_ppm: Option<f64>,
    pub result_chloride_ppm: Option<f64>,
    pub result_bicarbonate_ppm: Option<f64>,
    pub result_alkalinity: Option<f64>,
    pub result_residual_alk: Option<f64>,
    pub result_sulfate_to_chloride: Option<f64>,
    pub result_mash_ph: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AdjustmentRow {
    /// Reshapes a scanned row into the public [`Adjustment`], collapsing the
    /// nullable result columns into an `Option<Result>`.
    pub fn into_adjustment(self) -> Adjustment {
        let result = self.result_calcium_ppm.map(|ca| Result {
            calcium_ppm: ca,
            magnesium_ppm: self.result_magnesium_ppm.unwrap_or(0.0),
            sodium_ppm: self.result_sodium_ppm.unwrap_or(0.0),
            sulfate_ppm: self.result_sulfate_ppm.unwrap_or(0.0),
            chloride_ppm: self.result_chloride_ppm.unwrap_or(0.0),
            bicarbonate_ppm: self.result_bicarbonate_ppm.unwrap_or(0.0),
            alkalinity: self.result_alkalinity.unwrap_or(0.0),
            residual_alk: self.result_residual_alk.unwrap_or(0.0),
            sulfate_to_chloride: self.result_sulfate_to_chloride.unwrap_or(0.0),
            mash_ph: self.result_mash_ph.unwrap_or(0.0),
        });
        Adjustment {
            id: self.id,
            tenant_id: self.tenant_id,
            name: self.name,
            source_profile_id: self.source_profile_id,
            target_profile_id: self.target_profile_id,
            batch_id: self.batch_id,
            recipe_id: self.recipe_id,
            volume_liters: self.volume_liters,
            mineral_additions: self.mineral_additions,
            acid_additions: self.acid_additions,
            grain_additions: self.grain_additions,
            result,
            notes: self.notes,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Body for `POST /water-adjustments` and `PUT /water-adjustments/{id}`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateWaterAdjustmentRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub source_profile_id: Option<Uuid>,
    pub target_profile_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    #[validate(range(exclusive_min = 0.0))]
    pub volume_liters: f64,
    #[serde(default)]
    pub mineral_additions: Vec<MineralAddition>,
    #[serde(default)]
    pub acid_additions: Vec<AcidAddition>,
    #[serde(default)]
    pub grain_additions: Vec<GrainAddition>,
    pub notes: Option<String>,
}

/// Body for `PUT /water-adjustments/{id}`. All fields required (alias of create).
pub type UpdateWaterAdjustmentRequest = CreateWaterAdjustmentRequest;

/// Body for `PATCH /water-adjustments/{id}`. All fields optional.
#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchWaterAdjustmentRequest {
    pub name: Option<String>,
    pub source_profile_id: Option<Uuid>,
    pub target_profile_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub volume_liters: Option<f64>,
    pub mineral_additions: Option<Vec<MineralAddition>>,
    pub acid_additions: Option<Vec<AcidAddition>>,
    pub grain_additions: Option<Vec<GrainAddition>>,
    pub notes: Option<String>,
}

/// Body for `POST /water-adjustments/calculate` (stateless).
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CalculateRequest {
    pub source_profile_id: Option<Uuid>,
    pub source_profile: Option<InlineWaterProfile>,
    #[validate(range(exclusive_min = 0.0))]
    pub volume_liters: f64,
    #[serde(default)]
    pub mineral_additions: Vec<MineralAddition>,
    #[serde(default)]
    pub acid_additions: Vec<AcidAddition>,
    #[serde(default)]
    pub grain_additions: Vec<GrainAddition>,
}

/// Allows specifying a water profile inline in a [`CalculateRequest`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InlineWaterProfile {
    pub calcium_ppm: f64,
    pub magnesium_ppm: f64,
    pub sodium_ppm: f64,
    pub sulfate_ppm: f64,
    pub chloride_ppm: f64,
    pub bicarbonate_ppm: f64,
}

// ---- filters ----

/// List-query parameters for water profiles.
#[derive(Debug, Default)]
pub struct ProfileFilter {
    pub page: i32,
    pub page_size: i32,
    pub sort: String,
}

/// List-query parameters for water adjustments.
#[derive(Debug, Default)]
pub struct AdjustmentFilter {
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub page: i32,
    pub page_size: i32,
    pub sort: String,
}
