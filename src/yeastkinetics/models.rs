//! Yeast kinetics domain type, DTOs, and filters.
//!
//! Port of the Go `internal/yeastkinetics` types. `NUMERIC` columns
//! (`fermentation_temp_c`, `attenuation_pct`) are selected as `float8` so they
//! decode into `f64`. Validation ranges mirror the Go `validate:` tags exactly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// The reserved UUID for system-owned data.
pub const SYSTEM_TENANT_ID: Uuid = Uuid::nil();

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

/// A yeast fermentation kinetics row.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Kinetics {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub yeast_id: Uuid,
    pub fermentation_temp_c: f64,
    pub primary_fermentation_days: i32,
    pub conditioning_days: i32,
    pub lag_phase_hours: Option<i32>,
    pub attenuation_pct: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Payload for creating or replacing a yeast kinetics entry.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    pub yeast_id: Uuid,
    #[validate(range(min = 0.0, max = 40.0))]
    pub fermentation_temp_c: f64,
    #[validate(range(min = 1, max = 60))]
    pub primary_fermentation_days: i32,
    #[validate(range(min = 0, max = 365))]
    pub conditioning_days: i32,
    #[validate(range(min = 0, max = 168))]
    pub lag_phase_hours: Option<i32>,
    #[validate(range(min = 0.0, max = 100.0))]
    pub attenuation_pct: Option<f64>,
    #[validate(length(max = 500))]
    pub notes: Option<String>,
}

/// Partial-update payload. Pointer fields apply only if present.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchRequest {
    pub yeast_id: Option<Uuid>,
    pub fermentation_temp_c: Option<f64>,
    pub primary_fermentation_days: Option<i32>,
    pub conditioning_days: Option<i32>,
    pub lag_phase_hours: Option<i32>,
    pub attenuation_pct: Option<f64>,
    pub notes: Option<String>,
}

/// Query parameters for yeast kinetics listing.
#[derive(Debug, Default)]
pub struct ListFilter {
    pub yeast_id: Option<Uuid>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}
