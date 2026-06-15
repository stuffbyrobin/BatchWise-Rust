//! Packaging domain types and DTOs.
//!
//! Port of the Go `internal/packaging` types. `packaged_at` and
//! `best_before_date` are `DATE` columns rendered to `YYYY-MM-DD` strings;
//! `created_at`/`updated_at`/`moved_at` are `TIMESTAMPTZ`. Money is not used
//! here; quantities and volumes are integers.

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

/// A packaging run: a batch packaged into a specific container format.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PackagingRun {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub batch_id: Uuid,
    pub format: String,
    pub unit_volume_ml: i32,
    pub quantity: i32,
    pub lot_number: String,
    pub packaged_at: String,
    pub best_before_date: Option<String>,
    pub notes: Option<String>,
    pub stock_remaining: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A distribution movement of packaged product.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DistributionMovement {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub packaging_run_id: Uuid,
    pub movement_type: String,
    pub quantity: i32,
    pub from_location: String,
    pub to_location: String,
    pub order_id: Option<Uuid>,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub moved_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ---- request DTOs ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreatePackagingRunRequest {
    pub batch_id: Uuid,
    #[validate(custom(function = "validate_format"))]
    pub format: String,
    #[validate(range(min = 1))]
    pub unit_volume_ml: i32,
    #[validate(range(min = 1))]
    pub quantity: i32,
    #[validate(length(min = 1))]
    pub lot_number: String,
    #[validate(length(min = 1))]
    pub packaged_at: String,
    pub best_before_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchPackagingRunRequest {
    pub best_before_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateMovementRequest {
    pub packaging_run_id: Uuid,
    #[validate(custom(function = "validate_movement_type"))]
    pub movement_type: String,
    #[validate(range(min = 1))]
    pub quantity: i32,
    pub from_location: Option<String>,
    #[validate(length(min = 1))]
    pub to_location: String,
    pub order_id: Option<Uuid>,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub moved_at: Option<DateTime<Utc>>,
}

// ---- filters ----

#[derive(Debug, Default)]
pub struct ListPackagingRunsFilter {
    pub batch_id: Option<Uuid>,
    pub format: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Default)]
pub struct ListMovementsFilter {
    pub packaging_run_id: Option<Uuid>,
    pub order_id: Option<Uuid>,
    pub movement_type: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

// ---- enums / validators ----

const FORMATS: [&str; 7] = [
    "can",
    "bottle",
    "keg",
    "cask",
    "polypin",
    "bag_in_box",
    "other",
];

const MOVEMENT_TYPES: [&str; 6] = [
    "sale",
    "taproom_transfer",
    "internal_transfer",
    "sample",
    "return",
    "disposal",
];

/// Movement types that reduce available stock.
pub const OUTBOUND_MOVEMENTS: [&str; 5] = [
    "sale",
    "taproom_transfer",
    "internal_transfer",
    "sample",
    "disposal",
];

fn validate_format(v: &str) -> Result<(), ValidationError> {
    if FORMATS.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_format"))
    }
}

fn validate_movement_type(v: &str) -> Result<(), ValidationError> {
    if MOVEMENT_TYPES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_movement_type"))
    }
}
