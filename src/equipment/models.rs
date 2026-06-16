//! Equipment domain types and DTOs.
//!
//! Port of the Go `internal/equipment` types. `purchased_at` is a `DATE` column
//! rendered to a `YYYY-MM-DD` string; `last_performed_at`, `performed_at`,
//! `next_due_at`, `created_at`, `updated_at` are `TIMESTAMPTZ`. `cost_pence` is
//! money as `i64`. Computed fields (`overdue_schedule_count`,
//! `next_maintenance_due_at`, `next_due_at`, `days_until_due`, `is_overdue`) are
//! produced in SQL and aliased to the struct field names.

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

/// A tracked physical brewery asset.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Equipment {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub equipment_type: String,
    pub serial_number: Option<String>,
    pub location: Option<String>,
    pub status: String,
    /// `DATE` rendered via `to_char(... ,'YYYY-MM-DD')`.
    pub purchased_at: Option<String>,
    pub notes: Option<String>,
    /// Computed across the equipment's active schedules.
    pub overdue_schedule_count: i32,
    pub next_maintenance_due_at: Option<DateTime<Utc>>,
    /// Populated on single-equipment reads only; omitted from list responses.
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifetime_maintenance_cost_pence: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A recurring maintenance task attached to a piece of equipment.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MaintenanceSchedule {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub equipment_id: Uuid,
    pub task_name: String,
    pub interval_days: i32,
    pub last_performed_at: Option<DateTime<Utc>>,
    pub active: bool,
    pub notes: Option<String>,
    // Computed fields.
    pub next_due_at: DateTime<Utc>,
    pub days_until_due: i32,
    pub is_overdue: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A maintenance / service / calibration action that happened.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MaintenanceEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub equipment_id: Uuid,
    pub schedule_id: Option<Uuid>,
    pub event_type: String,
    pub performed_at: DateTime<Utc>,
    pub performed_by: Option<String>,
    pub cost_pence: Option<i64>,
    pub cost_currency: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A schedule that is overdue or due within a window, denormalised with its
/// equipment details for the cross-equipment feed.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MaintenanceDueItem {
    pub schedule_id: Uuid,
    pub equipment_id: Uuid,
    pub equipment_name: String,
    pub equipment_type: String,
    pub task_name: String,
    pub interval_days: i32,
    pub last_performed_at: Option<DateTime<Utc>>,
    pub next_due_at: DateTime<Utc>,
    pub days_until_due: i32,
    pub is_overdue: bool,
}

// ---- request DTOs ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateEquipmentRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 100))]
    pub equipment_type: String,
    #[validate(length(max = 255))]
    pub serial_number: Option<String>,
    #[validate(length(max = 255))]
    pub location: Option<String>,
    #[validate(custom(function = "validate_date_opt"))]
    pub purchased_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchEquipmentRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub equipment_type: Option<String>,
    #[validate(length(max = 255))]
    pub serial_number: Option<String>,
    #[validate(length(max = 255))]
    pub location: Option<String>,
    #[validate(custom(function = "validate_status"))]
    pub status: Option<String>,
    #[validate(custom(function = "validate_date_opt"))]
    pub purchased_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateScheduleRequest {
    #[validate(length(min = 1, max = 255))]
    pub task_name: String,
    #[validate(range(min = 1))]
    pub interval_days: i32,
    pub last_performed_at: Option<DateTime<Utc>>,
    pub active: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchScheduleRequest {
    #[validate(length(min = 1, max = 255))]
    pub task_name: Option<String>,
    #[validate(range(min = 1))]
    pub interval_days: Option<i32>,
    pub last_performed_at: Option<DateTime<Utc>>,
    pub active: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateEventRequest {
    #[validate(custom(function = "validate_event_type"))]
    pub event_type: String,
    pub schedule_id: Option<Uuid>,
    pub performed_at: Option<DateTime<Utc>>,
    #[validate(length(max = 255))]
    pub performed_by: Option<String>,
    #[validate(range(min = 0))]
    pub cost_pence: Option<i64>,
    #[validate(custom(function = "validate_iso_currency_present"))]
    pub cost_currency: Option<String>,
    pub notes: Option<String>,
}

// ---- filters ----

#[derive(Debug, Default)]
pub struct Filter {
    pub status: Option<String>,
    pub equipment_type: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

#[derive(Debug, Default)]
pub struct ScheduleFilter {
    pub active: Option<bool>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

#[derive(Debug, Default)]
pub struct EventFilter {
    pub event_type: Option<String>,
    pub schedule_id: Option<Uuid>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

#[derive(Debug)]
pub struct MaintenanceDueFilter {
    pub window_days: i64,
    pub overdue_only: bool,
    pub page: i64,
    pub page_size: i64,
}

// ---- enums / validators ----

/// Equipment status allowlist (Go `oneof=active retired`).
const STATUSES: [&str; 2] = ["active", "retired"];

/// Event-type allowlist (Go `oneof=service calibration repair inspection cleaning other`).
const EVENT_TYPES: [&str; 6] = [
    "service",
    "calibration",
    "repair",
    "inspection",
    "cleaning",
    "other",
];

/// Fixed ISO currency allowlist (matches the Go `iso_currency` validator).
const ISO_CURRENCIES: [&str; 14] = [
    "AED", "AUD", "CAD", "CHF", "CNY", "EUR", "GBP", "HKD", "JPY", "NOK", "NZD", "SEK", "SGD",
    "USD",
];

fn is_iso_currency(v: &str) -> bool {
    v.len() == 3 && v.chars().all(|c| c.is_ascii_uppercase()) && ISO_CURRENCIES.contains(&v)
}

fn validate_status(v: &str) -> Result<(), ValidationError> {
    if STATUSES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_status"))
    }
}

fn validate_event_type(v: &str) -> Result<(), ValidationError> {
    if EVENT_TYPES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_event_type"))
    }
}

/// Present currency (Go `omitempty,iso_currency`): empty string passes
/// `omitempty`; otherwise must be a known ISO code.
fn validate_iso_currency_present(v: &str) -> Result<(), ValidationError> {
    if v.is_empty() || is_iso_currency(v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_currency"))
    }
}

/// Optional date (Go `omitempty,datetime=2006-01-02`): when present, must parse
/// as `%Y-%m-%d`.
fn validate_date_opt(v: &str) -> Result<(), ValidationError> {
    if chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok() {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_date"))
    }
}
