//! Inventory domain types, DTOs, and filters.
//!
//! Port of the Go `internal/inventory` types. `NUMERIC` columns are selected as
//! `float8` so they map to `f64`; `best_before_date` is rendered to/parsed from
//! `YYYY-MM-DD` strings.

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
    /// Builds a page, computing `total_pages`.
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

/// A single inventory lot.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Ingredient {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub r#type: String,
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub lot_number: String,
    pub best_before_date: Option<String>,
    pub cost_pence: i64,
    pub cost_currency: String,
    pub supplier: Option<String>,
    pub origin: Option<String>,
    pub color_ebc: Option<f64>,
    pub alpha_acid_pct: Option<f64>,
    pub attenuation_pct: Option<f64>,
    pub allergens: Vec<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Immutable audit record of a stock change.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct StockMovement {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub ingredient_id: Uuid,
    pub amount_delta: f64,
    pub balance_after: f64,
    pub reference_type: String,
    pub reference_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// One aggregated row from the inventory summary.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SummaryRow {
    pub r#type: String,
    pub name: String,
    pub unit: String,
    pub total_amount: f64,
    pub lot_count: i64,
    pub earliest_best_before_date: Option<String>,
    pub weighted_avg_cost_pence_per_unit: i64,
}

/// One lot's contribution to a deduct result.
#[derive(Debug, Clone, Serialize)]
pub struct AllocationEntry {
    pub lot_id: Uuid,
    pub lot_number: String,
    pub best_before_date: Option<String>,
    pub amount_deducted: f64,
    pub remaining_in_lot: f64,
}

/// Returned by a successful deduct call.
#[derive(Debug, Clone, Serialize)]
pub struct DeductResult {
    pub requested_amount: f64,
    pub deducted_amount: f64,
    pub unit: String,
    pub allocations: Vec<AllocationEntry>,
    pub warning: Option<String>,
}

// ---- Request DTOs ----

/// Payload for creating (or, via PUT, replacing) an ingredient lot.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    #[validate(custom(function = "validate_type"))]
    pub r#type: String,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub amount: f64,
    #[validate(custom(function = "validate_unit"))]
    pub unit: String,
    #[validate(custom(function = "validate_lot_number"))]
    pub lot_number: String,
    pub best_before_date: Option<String>,
    #[serde(default)]
    #[validate(range(min = 0))]
    pub cost_pence: i64,
    #[serde(default)]
    #[validate(custom(function = "validate_currency_opt"))]
    pub cost_currency: String,
    #[validate(length(max = 255))]
    pub supplier: Option<String>,
    #[validate(length(max = 100))]
    pub origin: Option<String>,
    #[validate(range(min = 0.0))]
    pub color_ebc: Option<f64>,
    #[validate(range(min = 0.0, max = 100.0))]
    pub alpha_acid_pct: Option<f64>,
    #[validate(range(min = 0.0, max = 100.0))]
    pub attenuation_pct: Option<f64>,
    #[serde(default)]
    pub allergens: Vec<String>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
}

/// Partial-update payload for an ingredient lot.
#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchRequest {
    pub r#type: Option<String>,
    pub name: Option<String>,
    pub amount: Option<f64>,
    pub unit: Option<String>,
    pub lot_number: Option<String>,
    pub best_before_date: Option<String>,
    pub cost_pence: Option<i64>,
    pub cost_currency: Option<String>,
    pub supplier: Option<String>,
    pub origin: Option<String>,
    pub color_ebc: Option<f64>,
    pub alpha_acid_pct: Option<f64>,
    pub attenuation_pct: Option<f64>,
    pub allergens: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Payload for appending stock to an existing lot.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StockInRequest {
    #[validate(range(exclusive_min = 0.0))]
    pub amount: f64,
    pub notes: Option<String>,
    pub cost_pence: Option<i64>,
}

/// Payload for a FIFO stock deduction.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DeductRequest {
    #[validate(custom(function = "validate_type"))]
    pub r#type: String,
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub amount: f64,
    #[validate(custom(function = "validate_unit"))]
    pub unit: String,
    #[serde(default)]
    pub reference_type: String,
    pub reference_id: Option<Uuid>,
    #[validate(length(max = 500))]
    pub notes: Option<String>,
    pub preferred_lot_id: Option<Uuid>,
}

// ---- Filters ----

/// List-query parameters for ingredient lots.
#[derive(Debug, Default)]
pub struct ListFilter {
    pub r#type: Option<String>,
    pub name: Option<String>,
    pub lot_number: Option<String>,
    pub expiring_before: Option<String>,
    pub expiring_within_days: Option<i32>,
    pub out_of_stock: bool,
    pub sort: String,
    pub page: i64,
    pub page_size: i64,
}

/// Filter parameters for the inventory summary.
#[derive(Debug, Default)]
pub struct SummaryFilter {
    pub r#type: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// List-query parameters for stock movements.
#[derive(Debug, Default)]
pub struct MovementFilter {
    pub ingredient_id: Option<Uuid>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub sort: String,
    pub page: i64,
    pub page_size: i64,
}

// ---- Custom validators (mirror the Go `validate:` tags) ----

const TYPES: [&str; 6] = [
    "fermentable",
    "hop",
    "yeast",
    "adjunct",
    "chemical",
    "other",
];
const UNITS: [&str; 5] = ["kg", "g", "L", "mL", "count"];

fn validate_type(v: &str) -> Result<(), ValidationError> {
    if TYPES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_type"))
    }
}

fn validate_unit(v: &str) -> Result<(), ValidationError> {
    if UNITS.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_unit"))
    }
}

/// `lot_number`: 1–100 chars, alphanumeric plus hyphens.
fn validate_lot_number(v: &str) -> Result<(), ValidationError> {
    if (1..=100).contains(&v.len()) && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_lot_number"))
    }
}

/// `cost_currency`: empty (defaulted later) or a 3-letter ISO code.
fn validate_currency_opt(v: &str) -> Result<(), ValidationError> {
    if v.is_empty() || (v.len() == 3 && v.chars().all(|c| c.is_ascii_uppercase())) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_currency"))
    }
}
