//! Batch domain types, recipe snapshot, and DTOs.
//!
//! Port of the Go `internal/batch` types. A batch is an immutable snapshot of a
//! recipe at brew time; the snapshot is stored as JSONB.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::recipe::models::{Fermentable, Hop, MashStep, Yeast};

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

/// Recipe state captured at batch creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRecipeSnapshot {
    pub schema_version: i32,
    pub recipe_id: Uuid,
    pub name: String,
    pub r#type: String,
    pub batch_size_liters: f64,
    pub boil_size_liters: Option<f64>,
    pub boil_time_minutes: Option<i32>,
    pub efficiency_pct: Option<f64>,
    pub calc_og: Option<f64>,
    pub calc_fg: Option<f64>,
    pub calc_abv_pct: Option<f64>,
    pub calc_ibu: Option<f64>,
    pub calc_color_ebc: Option<f64>,
    pub fermentables: Vec<Fermentable>,
    pub hops: Vec<Hop>,
    pub yeasts: Vec<Yeast>,
    pub mash_steps: Vec<MashStep>,
}

/// A brewing batch.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Batch {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub recipe_id: Option<Uuid>,
    pub fermenter_id: Option<Uuid>,
    pub batch_number: String,
    pub name: String,
    pub status: String,
    pub brew_date: Option<String>,
    pub package_date: Option<String>,
    pub target_og: Option<f64>,
    pub actual_og: Option<f64>,
    pub target_fg: Option<f64>,
    pub actual_fg: Option<f64>,
    pub actual_volume_liters: Option<f64>,
    pub notes: Option<String>,
    pub duty_status: String,
    #[sqlx(json)]
    pub batch_recipe_snapshot: BatchRecipeSnapshot,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Ingredient deduction recorded against a batch.
#[derive(Debug, Clone, Serialize)]
pub struct BatchIngredient {
    pub batch_id: Uuid,
    pub ingredient_id: Uuid,
    pub amount_deducted: f64,
    pub unit: String,
    pub cost_pence: i64,
}

// ---- request DTOs ----

/// Payload for creating a batch.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    pub recipe_id: Uuid,
    pub fermenter_id: Option<Uuid>,
    #[validate(length(min = 1, max = 50))]
    pub batch_number: String,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub brew_date: Option<String>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
    #[validate(custom(function = "validate_status_opt"))]
    pub initial_status: Option<String>,
}

/// Payload for updating a batch's mutable fields.
#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest {
    #[validate(length(max = 255))]
    pub name: Option<String>,
    pub fermenter_id: Option<Uuid>,
    pub brew_date: Option<String>,
    pub package_date: Option<String>,
    pub target_og: Option<f64>,
    pub actual_og: Option<f64>,
    pub target_fg: Option<f64>,
    pub actual_fg: Option<f64>,
    pub actual_volume_liters: Option<f64>,
    pub notes: Option<String>,
}

/// Payload for a status transition.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct TransitionRequest {
    #[validate(length(min = 1))]
    pub to_status: String,
}

/// Payload to replace the ingredient lists in the recipe snapshot.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchIngredientsRequest {
    #[serde(default)]
    pub fermentables: Vec<Fermentable>,
    #[serde(default)]
    pub hops: Vec<Hop>,
    #[serde(default)]
    pub yeasts: Vec<Yeast>,
}

/// Filters for listing batches.
#[derive(Debug, Default)]
pub struct ListFilter {
    pub status: Option<String>,
    pub recipe_id: Option<Uuid>,
    pub brew_date_from: Option<String>,
    pub brew_date_to: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

/// Result of creating a batch: the batch plus the generated calendar events.
#[derive(Debug, Serialize)]
pub struct CreateResult {
    pub batch: Batch,
    pub generated_calendar_events: Vec<crate::calendar::models::Event>,
}

const STATUSES: [&str; 8] = [
    "planned",
    "brewing",
    "fermenting",
    "conditioning",
    "packaging",
    "completed",
    "cancelled",
    "spoiled",
];

fn validate_status_opt(v: &str) -> Result<(), ValidationError> {
    if STATUSES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_status"))
    }
}
