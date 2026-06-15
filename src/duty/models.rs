//! Duty-return domain types, DTOs, and filters.
//!
//! Port of the Go `internal/compliance/duty` types. Money is `i64` pence
//! (`BIGINT`). `NUMERIC` columns (`total_volume_liters`,
//! `sbr_annual_production_hl_pa`, `sbr_relief_rate_pct`) are selected as
//! `float8`; `DATE` columns (`period_start`, `period_end`) are rendered with
//! `to_char(..., 'YYYY-MM-DD')`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

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

/// A compiled Beer Duty return for a calendar period.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Return {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// ISO date (`YYYY-MM-DD`).
    pub period_start: String,
    /// ISO date (`YYYY-MM-DD`).
    pub period_end: String,
    pub status: String,
    pub event_count: i32,
    pub total_volume_liters: f64,
    pub gross_duty_pence: i64,
    pub sbr_annual_production_hl_pa: f64,
    pub sbr_relief_rate_pct: f64,
    pub sbr_relief_pence: i64,
    pub net_duty_pence: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- request DTOs ----

/// Body for `POST /duty-returns/compile`.
#[derive(Debug, Clone, Deserialize, validator::Validate)]
#[serde(deny_unknown_fields)]
pub struct CompileRequest {
    #[validate(length(min = 1))]
    pub period_start: String,
    #[validate(length(min = 1))]
    pub period_end: String,
}

/// Body for `PATCH /duty-returns/{id}`.
#[derive(Debug, Clone, Default, Deserialize, validator::Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchRequest {
    pub status: Option<String>,
}

/// Filters for `GET /duty-returns`.
#[derive(Debug, Default)]
pub struct ReturnFilter {
    pub status: Option<String>,
    /// ISO date — lower bound on `period_start`.
    pub from_date: Option<String>,
    /// ISO date — upper bound on `period_start`.
    pub to_date: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: Option<String>,
}
