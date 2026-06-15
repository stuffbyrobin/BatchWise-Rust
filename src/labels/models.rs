//! Label-record domain types, DTOs, and filters.
//!
//! Port of the Go `internal/compliance/labels` types. The `allergens` column is
//! a Postgres `TEXT[]`. `NUMERIC` columns (`abv_percent`, `energy_kj_per_100ml`,
//! `energy_kcal_per_100ml`, `alcohol_units_per_serving`) are selected as
//! `float8`; the `DATE` column (`best_before_date`) is rendered with
//! `to_char(..., 'YYYY-MM-DD')`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Generic paginated response envelope.
///
/// The Go `LabelRecordList` shape is `{ items, total, page, page_size }`; this
/// envelope adds `total_pages` consistently with the other ported modules.
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

// ---- domain types ----

/// A UK label-compliance record for a batch.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LabelRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub batch_id: Uuid,
    pub status: String,

    pub product_name: String,
    pub abv_percent: f64,
    pub allergens: Vec<String>,
    pub net_volume_ml: i32,
    pub responsible_party: String,
    pub country_of_origin: String,
    /// ISO date (`YYYY-MM-DD`), null until set.
    pub best_before_date: Option<String>,
    pub lot_identifier: String,

    pub ingredient_list: Option<String>,
    pub energy_kj_per_100ml: Option<f64>,
    pub energy_kcal_per_100ml: Option<f64>,
    pub alcohol_units_per_serving: Option<f64>,
    pub serving_volume_ml: Option<i32>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- request DTOs ----

/// Body for `POST /label-records`.
#[derive(Debug, Clone, Deserialize, validator::Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    pub batch_id: Uuid,
    #[validate(range(min = 1))]
    pub net_volume_ml: i32,
    #[validate(range(min = 1))]
    pub serving_volume_ml: Option<i32>,
}

/// Body for `PATCH /label-records/{id}`.
#[derive(Debug, Clone, Default, Deserialize, validator::Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchRequest {
    pub product_name: Option<String>,
    #[validate(range(min = 0.0))]
    pub abv_percent: Option<f64>,
    pub allergens: Option<Vec<String>>,
    #[validate(range(min = 1))]
    pub net_volume_ml: Option<i32>,
    pub responsible_party: Option<String>,
    #[validate(length(equal = 2))]
    pub country_of_origin: Option<String>,
    pub best_before_date: Option<String>,
    pub lot_identifier: Option<String>,
    pub ingredient_list: Option<String>,
    pub energy_kj_per_100ml: Option<f64>,
    pub energy_kcal_per_100ml: Option<f64>,
    pub alcohol_units_per_serving: Option<f64>,
    #[validate(range(min = 1))]
    pub serving_volume_ml: Option<i32>,
    #[validate(custom(function = "validate_status"))]
    pub status: Option<String>,
}

/// Validates the `status` enum (`draft` | `approved`), mirroring the Go
/// `oneof=draft approved` tag.
fn validate_status(status: &str) -> Result<(), validator::ValidationError> {
    if status == "draft" || status == "approved" {
        Ok(())
    } else {
        Err(validator::ValidationError::new("oneof"))
    }
}

/// Filters for `GET /label-records`.
#[derive(Debug, Default)]
pub struct ListFilter {
    pub batch_id: Option<Uuid>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: Option<String>,
}
