//! Reporting domain types, DTOs, and filters.
//!
//! Port of the Go `internal/reporting` types. Money is `i64` pence (`BIGINT`).
//! `NUMERIC` columns (`rate_value`) are selected as `float8`; `DATE` columns are
//! rendered with `to_char(..., 'YYYY-MM-DD')`; `report_data` is `JSONB`.

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

// ---- domain types ----

/// A cost rate record.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Rate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub rate_type: String,
    pub rate_name: String,
    pub unit: String,
    pub rate_value: f64,
    pub currency: String,
    /// ISO date (`YYYY-MM-DD`).
    pub effective_from: String,
    /// ISO date or null.
    pub effective_to: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The computed cost breakdown for a batch. `total_cost_pence` is a DB-generated
/// column (read, never written); `margin_pence` is derived in code.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BatchCost {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub batch_id: Uuid,
    pub ingredient_cost_pence: i64,
    pub energy_cost_pence: i64,
    pub labor_cost_pence: i64,
    pub water_cost_pence: i64,
    pub overhead_cost_pence: i64,
    pub estimated_duty_pence: i64,
    pub total_cost_pence: i64,
    pub revenue_pence: i64,
    pub margin_pence: i64,
    pub cost_per_liter_pence: Option<i64>,
    pub cost_per_unit_pence: Option<i64>,
    pub computed_at: DateTime<Utc>,
}

/// A per-batch summary row for the profitability report.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProfitabilityRow {
    pub batch_id: Uuid,
    pub batch_name: String,
    pub created_at: DateTime<Utc>,
    pub total_cost_pence: i64,
    pub revenue_pence: i64,
    pub margin_pence: i64,
}

/// A stored cost report.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Report {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub report_type: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    #[sqlx(json)]
    pub report_data: serde_json::Value,
    pub generated_at: DateTime<Utc>,
}

// ---- request DTOs ----

/// Body for `POST /cost-rates` and `PUT /cost-rates/{id}`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRateRequest {
    #[validate(custom(function = "validate_rate_type"))]
    pub rate_type: String,
    #[validate(length(min = 1))]
    pub rate_name: String,
    #[validate(length(min = 1))]
    pub unit: String,
    #[validate(custom(function = "validate_rate_value"))]
    pub rate_value: f64,
    #[serde(default)]
    pub currency: Option<String>,
    #[validate(length(min = 1))]
    pub effective_from: String,
    pub effective_to: Option<String>,
    pub notes: Option<String>,
}

/// Body for `PATCH /cost-rates/{id}`.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchRateRequest {
    pub rate_name: Option<String>,
    pub unit: Option<String>,
    pub rate_value: Option<f64>,
    pub currency: Option<String>,
    pub effective_from: Option<String>,
    pub effective_to: Option<String>,
    pub notes: Option<String>,
}

/// Filters for `GET /cost-rates`.
#[derive(Debug, Default)]
pub struct RateFilter {
    pub rate_type: Option<String>,
    /// ISO date — filters rates valid on this date.
    pub effective_on: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// Body for `POST /batch-costs/compute`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ComputeBatchCostRequest {
    pub batch_id: Uuid,
    pub energy_kwh: Option<f64>,
    pub labor_hours: Option<f64>,
    pub water_liters: Option<f64>,
    pub overhead_pence: Option<i64>,
}

/// Filters for `GET /batch-costs`.
#[derive(Debug, Default)]
pub struct BatchCostFilter {
    pub batch_id: Option<Uuid>,
    pub page: i64,
    pub page_size: i64,
}

/// Body for `POST /cost-reports/generate`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GenerateReportRequest {
    #[validate(custom(function = "validate_report_type"))]
    pub report_type: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
}

/// Filters for `GET /cost-reports`.
#[derive(Debug, Default)]
pub struct ReportFilter {
    pub report_type: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

// ---- validators ----

fn validate_rate_type(v: &str) -> Result<(), ValidationError> {
    if ["energy", "labor", "water", "duty", "overhead"].contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_rate_type"))
    }
}

/// Mirrors Go's `validate:"required"` on a numeric: zero is rejected.
fn validate_rate_value(v: f64) -> Result<(), ValidationError> {
    if v == 0.0 {
        Err(ValidationError::new("required"))
    } else {
        Ok(())
    }
}

fn validate_report_type(v: &str) -> Result<(), ValidationError> {
    if ["batch", "recipe", "period", "inventory", "profitability"].contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_report_type"))
    }
}
