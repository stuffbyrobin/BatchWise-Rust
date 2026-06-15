//! Fermentation domain types and DTOs.
//!
//! Port of the Go `internal/fermentation` types. `recorded_at`/`created_at` are
//! `TIMESTAMPTZ`; `gravity`, `temp_c` and `ph` are `NUMERIC` (decoded as `f64`).

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
        let total_pages = if total > 0 && page_size > 0 {
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

/// A single fermentation measurement logged against a batch.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Reading {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub batch_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub stage: String,
    pub gravity: Option<f64>,
    pub temp_c: Option<f64>,
    pub ph: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ---- request DTOs ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateReadingRequest {
    pub recorded_at: Option<DateTime<Utc>>,
    #[validate(custom(function = "validate_stage"))]
    pub stage: Option<String>,
    #[validate(range(exclusive_min = 0.0))]
    pub gravity: Option<f64>,
    pub temp_c: Option<f64>,
    #[validate(range(min = 0.0, max = 14.0))]
    pub ph: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchReadingRequest {
    pub recorded_at: Option<DateTime<Utc>>,
    #[validate(custom(function = "validate_stage"))]
    pub stage: Option<String>,
    #[validate(range(exclusive_min = 0.0))]
    pub gravity: Option<f64>,
    pub temp_c: Option<f64>,
    #[validate(range(min = 0.0, max = 14.0))]
    pub ph: Option<f64>,
    pub notes: Option<String>,
}

// ---- filter ----

#[derive(Debug, Default)]
pub struct ReadingFilter {
    pub stage: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

// ---- enums / validators ----

const STAGES: [&str; 5] = ["primary", "secondary", "conditioning", "lagering", "other"];

fn validate_stage(v: &str) -> Result<(), ValidationError> {
    if STAGES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_stage"))
    }
}
