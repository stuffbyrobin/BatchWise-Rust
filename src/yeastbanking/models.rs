//! Yeast banking domain types and DTOs.
//!
//! Port of the Go `internal/yeastbanking` types. `harvested_at`, `started_at`,
//! `completed_at`, `created_at`, `updated_at` are `TIMESTAMPTZ` columns;
//! `viability_percent`, `quantity_ml`, `storage_temp_c`, `volume_ml` are
//! `NUMERIC` (decoded as `f64`). `days_in_storage` is NOT a column — it is
//! computed in code and skipped by `FromRow`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

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

// ---- domain types ----

/// A physical yeast culture held in the tenant's yeast bank.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct YeastBankEntry {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub library_yeast_id: Option<Uuid>,
    pub generation: i32,
    pub harvested_at: Option<DateTime<Utc>>,
    pub viability_percent: Option<f64>,
    pub quantity_ml: Option<f64>,
    pub storage_temp_c: Option<f64>,
    pub location: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    #[sqlx(skip)]
    pub days_in_storage: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single propagation (step-up) event for a yeast bank entry.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Propagation {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub yeast_bank_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub volume_ml: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Computes `days_in_storage` from `harvested_at`, mirroring Go's
/// `int(time.Since(h).Hours()/24)` (integer-truncated whole days).
pub fn compute_days_in_storage(harvested_at: Option<DateTime<Utc>>) -> Option<i32> {
    harvested_at.map(|h| ((Utc::now() - h).num_seconds() / 86_400) as i32)
}

// ---- request DTOs ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateYeastBankRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub library_yeast_id: Option<Uuid>,
    pub generation: Option<i32>,
    pub harvested_at: Option<DateTime<Utc>>,
    #[validate(range(min = 0.0, max = 100.0))]
    pub viability_percent: Option<f64>,
    #[validate(range(min = 0.0))]
    pub quantity_ml: Option<f64>,
    pub storage_temp_c: Option<f64>,
    pub location: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchYeastBankRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub library_yeast_id: Option<Uuid>,
    #[validate(range(min = 0.0, max = 100.0))]
    pub viability_percent: Option<f64>,
    #[validate(range(min = 0.0))]
    pub quantity_ml: Option<f64>,
    pub storage_temp_c: Option<f64>,
    pub location: Option<String>,
    #[validate(custom(function = "validate_status"))]
    pub status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HarvestRequest {
    pub harvested_at: Option<DateTime<Utc>>,
    #[validate(range(min = 0.0, max = 100.0))]
    pub viability_percent: Option<f64>,
    #[validate(range(min = 0.0))]
    pub quantity_ml: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreatePropagationRequest {
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    #[validate(range(min = 0.0))]
    pub volume_ml: Option<f64>,
    pub batch_id: Option<Uuid>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchPropagationRequest {
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    #[validate(range(min = 0.0))]
    pub volume_ml: Option<f64>,
    pub batch_id: Option<Uuid>,
    pub notes: Option<String>,
}

// ---- filters ----

#[derive(Debug, Default)]
pub struct YeastBankFilter {
    pub status: Option<String>,
    pub library_yeast_id: Option<Uuid>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

// ---- validators ----

/// PATCH status allowlist (Go `oneof=active depleted discarded`).
const STATUSES: [&str; 3] = ["active", "depleted", "discarded"];

fn validate_status(v: &str) -> Result<(), validator::ValidationError> {
    if STATUSES.contains(&v) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_status"))
    }
}
