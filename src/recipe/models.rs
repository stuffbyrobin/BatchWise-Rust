//! Recipe domain types, nested ingredient types, DTOs, and filters.
//!
//! Port of the Go `internal/recipe` types. `NUMERIC` columns are selected as
//! `float8`; nested arrays (fermentables/hops/yeasts/mash steps) are children of
//! a recipe.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

/// Generic paginated response envelope.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 && total > 0 {
            (total + page_size - 1) / page_size
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

/// Top-level recipe row (no nested arrays).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Recipe {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub r#type: String,
    pub style_id: Option<Uuid>,
    pub equipment_profile_id: Option<Uuid>,
    pub mash_profile_id: Option<Uuid>,
    pub batch_size_liters: f64,
    pub boil_size_liters: Option<f64>,
    pub boil_time_minutes: Option<i32>,
    pub efficiency_pct: Option<f64>,
    pub calc_og: Option<f64>,
    pub calc_fg: Option<f64>,
    pub calc_abv_pct: Option<f64>,
    pub calc_ibu: Option<f64>,
    pub calc_color_ebc: Option<f64>,
    pub tasting_aroma: Option<String>,
    pub tasting_flavour: Option<String>,
    pub tasting_mouthfeel: Option<String>,
    pub tasting_finish: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Full recipe including nested ingredient arrays.
#[derive(Debug, Clone, Serialize)]
pub struct RecipeWithIngredients {
    #[serde(flatten)]
    pub recipe: Recipe,
    pub fermentables: Vec<Fermentable>,
    pub hops: Vec<Hop>,
    pub yeasts: Vec<Yeast>,
    pub mash_steps: Vec<MashStep>,
}

/// A single fermentable addition.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Fermentable {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub step_order: i32,
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub color_ebc: Option<f64>,
    pub potential_ppg: Option<f64>,
    pub r#type: Option<String>,
    pub addition: Option<String>,
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_lot_id: Option<Uuid>,
}

/// A single hop addition.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Hop {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub step_order: i32,
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub alpha_acid_pct: f64,
    pub boil_time_minutes: f64,
    pub form: Option<String>,
    pub r#use: Option<String>,
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_lot_id: Option<Uuid>,
}

/// A single yeast addition.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Yeast {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub yeast_id: Option<Uuid>,
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub attenuation_pct: Option<f64>,
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_lot_id: Option<Uuid>,
}

/// A single mash step.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MashStep {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub step_order: i32,
    pub step_type: String,
    pub target_temp_c: f64,
    pub hold_minutes: i32,
    pub infusion_volume_liters: Option<f64>,
}

/// Physics-derived fields updated after every write.
#[derive(Debug, Clone, Copy)]
pub struct CalculatedValues {
    pub calc_og: f64,
    pub calc_fg: f64,
    pub calc_abv_pct: f64,
    pub calc_ibu: f64,
    pub calc_color_ebc: f64,
}

// ---- Request DTOs ----

/// One fermentable in a create/replace request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FermentableInput {
    #[validate(range(min = 1))]
    pub step_order: i32,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub amount: f64,
    #[validate(custom(function = "validate_ferm_unit"))]
    pub unit: String,
    #[validate(range(min = 0.0))]
    pub color_ebc: Option<f64>,
    #[validate(range(min = 0.0))]
    pub potential_ppg: Option<f64>,
    pub r#type: Option<String>,
    pub addition: Option<String>,
}

/// One hop addition in a create/replace request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HopInput {
    #[validate(range(min = 1))]
    pub step_order: i32,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub amount: f64,
    #[validate(custom(function = "validate_hop_unit"))]
    pub unit: String,
    #[validate(range(exclusive_min = 0.0))]
    pub alpha_acid_pct: f64,
    #[validate(range(min = 0.0))]
    pub boil_time_minutes: f64,
    #[validate(custom(function = "validate_hop_form"))]
    pub form: Option<String>,
    #[validate(custom(function = "validate_hop_use"))]
    pub r#use: Option<String>,
}

/// One yeast addition in a create/replace request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct YeastInput {
    pub yeast_id: Option<Uuid>,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub amount: f64,
    #[validate(custom(function = "validate_yeast_unit"))]
    pub unit: String,
    #[validate(range(min = 0.0, max = 100.0))]
    pub attenuation_pct: Option<f64>,
}

/// One mash step in a create/replace request.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct MashStepInput {
    #[validate(range(min = 1))]
    pub step_order: i32,
    #[validate(custom(function = "validate_mash_step_type"))]
    pub step_type: String,
    pub target_temp_c: f64,
    #[validate(range(min = 1))]
    pub hold_minutes: i32,
    #[validate(range(exclusive_min = 0.0))]
    pub infusion_volume_liters: Option<f64>,
}

/// Payload for creating or replacing a recipe.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(custom(function = "validate_recipe_type"))]
    pub r#type: String,
    pub style_id: Option<Uuid>,
    pub equipment_profile_id: Option<Uuid>,
    pub mash_profile_id: Option<Uuid>,
    #[validate(range(exclusive_min = 0.0))]
    pub batch_size_liters: f64,
    #[validate(range(exclusive_min = 0.0))]
    pub boil_size_liters: Option<f64>,
    #[validate(range(min = 0))]
    pub boil_time_minutes: Option<i32>,
    #[validate(range(min = 0.0, max = 100.0))]
    pub efficiency_pct: Option<f64>,
    #[validate(length(max = 1000))]
    pub tasting_aroma: Option<String>,
    #[validate(length(max = 1000))]
    pub tasting_flavour: Option<String>,
    #[validate(length(max = 1000))]
    pub tasting_mouthfeel: Option<String>,
    #[validate(length(max = 1000))]
    pub tasting_finish: Option<String>,
    #[validate(length(max = 5000))]
    pub notes: Option<String>,
    #[validate(nested)]
    #[serde(default)]
    pub fermentables: Option<Vec<FermentableInput>>,
    #[validate(nested)]
    #[serde(default)]
    pub hops: Option<Vec<HopInput>>,
    #[validate(nested)]
    #[serde(default)]
    pub yeasts: Option<Vec<YeastInput>>,
    #[validate(nested)]
    #[serde(default)]
    pub mash_steps: Option<Vec<MashStepInput>>,
}

/// Partial-update payload. Pointer fields apply only if present; nested arrays
/// replace all rows when present.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchRequest {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub style_id: Option<Uuid>,
    pub equipment_profile_id: Option<Uuid>,
    pub mash_profile_id: Option<Uuid>,
    pub batch_size_liters: Option<f64>,
    pub boil_size_liters: Option<f64>,
    pub boil_time_minutes: Option<i32>,
    pub efficiency_pct: Option<f64>,
    pub tasting_aroma: Option<String>,
    pub tasting_flavour: Option<String>,
    pub tasting_mouthfeel: Option<String>,
    pub tasting_finish: Option<String>,
    pub notes: Option<String>,
    #[validate(nested)]
    pub fermentables: Option<Vec<FermentableInput>>,
    #[validate(nested)]
    pub hops: Option<Vec<HopInput>>,
    #[validate(nested)]
    pub yeasts: Option<Vec<YeastInput>>,
    #[validate(nested)]
    pub mash_steps: Option<Vec<MashStepInput>>,
}

/// Payload for BeerXML or Brewfather import.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
    #[validate(custom(function = "validate_import_format"))]
    pub format: String,
    #[validate(length(min = 1))]
    pub data: String,
}

/// Query parameters for recipe listing.
#[derive(Debug, Default)]
pub struct ListFilter {
    pub name: Option<String>,
    pub style_id: Option<Uuid>,
    pub r#type: Option<String>,
    pub sort: String,
    pub page: i64,
    pub page_size: i64,
}

// ---- validators ----

fn one_of(v: &str, allowed: &[&str], code: &'static str) -> Result<(), ValidationError> {
    if allowed.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new(code))
    }
}

fn validate_recipe_type(v: &str) -> Result<(), ValidationError> {
    one_of(
        v,
        &[
            "all_grain",
            "extract",
            "partial_mash",
            "cider",
            "mead",
            "other",
        ],
        "invalid_type",
    )
}
fn validate_ferm_unit(v: &str) -> Result<(), ValidationError> {
    one_of(v, &["kg", "g"], "invalid_unit")
}
fn validate_hop_unit(v: &str) -> Result<(), ValidationError> {
    one_of(v, &["g", "kg"], "invalid_unit")
}
fn validate_yeast_unit(v: &str) -> Result<(), ValidationError> {
    one_of(v, &["g", "mL", "count"], "invalid_unit")
}
fn validate_hop_form(v: &str) -> Result<(), ValidationError> {
    one_of(v, &["pellet", "leaf", "extract"], "invalid_form")
}
fn validate_hop_use(v: &str) -> Result<(), ValidationError> {
    one_of(
        v,
        &["boil", "whirlpool", "dry-hop", "first-wort", "mash"],
        "invalid_use",
    )
}
fn validate_mash_step_type(v: &str) -> Result<(), ValidationError> {
    one_of(
        v,
        &["infusion", "temperature", "decoction"],
        "invalid_step_type",
    )
}
fn validate_import_format(v: &str) -> Result<(), ValidationError> {
    one_of(v, &["beerxml", "brewfather"], "invalid_format")
}
