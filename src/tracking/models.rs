//! Container-tracking domain types and DTOs.
//!
//! Port of the Go `internal/tracking` types. `NUMERIC` columns are selected as
//! `float8`; dates are rendered to `YYYY-MM-DD` strings.

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

/// A container asset (keg, cask, etc.).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Asset {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub asset_number: String,
    pub container_type: String,
    pub capacity_liters: f64,
    pub deposit_pence: i64,
    pub status: String,
    pub current_batch_id: Option<Uuid>,
    pub current_customer_name: Option<String>,
    pub last_fill_date: Option<String>,
    pub last_return_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A container event-log entry.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Log {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub container_id: Uuid,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub batch_id: Option<Uuid>,
    pub customer_name: Option<String>,
    pub notes: Option<String>,
    pub logged_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Generated QR-code result.
#[derive(Debug, Serialize)]
pub struct QrResult {
    pub container_id: String,
    pub variant: String,
    pub payload: String,
    pub png_base64: String,
}

// ---- request DTOs ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateAssetRequest {
    #[validate(length(min = 1))]
    pub asset_number: String,
    #[validate(custom(function = "validate_container_type"))]
    pub container_type: String,
    #[validate(range(exclusive_min = 0.0))]
    pub capacity_liters: f64,
    #[serde(default)]
    pub deposit_pence: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateAssetRequest {
    #[validate(length(min = 1))]
    pub asset_number: String,
    #[validate(custom(function = "validate_container_type"))]
    pub container_type: String,
    #[validate(range(exclusive_min = 0.0))]
    pub capacity_liters: f64,
    #[serde(default)]
    pub deposit_pence: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchAssetRequest {
    pub asset_number: Option<String>,
    pub container_type: Option<String>,
    pub capacity_liters: Option<f64>,
    pub deposit_pence: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FillRequest {
    pub batch_id: Option<Uuid>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DeliverRequest {
    #[validate(length(min = 1))]
    pub customer_name: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ReturnRequest {
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SetStatusRequest {
    #[validate(custom(function = "validate_status"))]
    pub to_status: String,
    pub notes: Option<String>,
}

// ---- filters ----

#[derive(Debug, Default)]
pub struct AssetFilter {
    pub status: Option<String>,
    pub container_type: Option<String>,
    pub current_batch_id: Option<Uuid>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

#[derive(Debug, Default)]
pub struct LogFilter {
    pub container_id: Option<Uuid>,
    pub event_type: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

const CONTAINER_TYPES: [&str; 7] = [
    "keg",
    "cask",
    "firkin",
    "bottle_case",
    "ibc",
    "tank",
    "other",
];
const STATUSES: [&str; 6] = [
    "empty",
    "filled",
    "delivered",
    "returned",
    "lost",
    "retired",
];

fn validate_container_type(v: &str) -> Result<(), ValidationError> {
    if CONTAINER_TYPES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_container_type"))
    }
}

fn validate_status(v: &str) -> Result<(), ValidationError> {
    if STATUSES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_status"))
    }
}
