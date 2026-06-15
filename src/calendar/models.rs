//! Calendar event domain types, DTOs, and filters.
//!
//! Port of the Go `internal/calendar` types. Timestamps are `DateTime<Utc>`;
//! `event_type` and `status` are constrained with `oneof`-style validators.

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

/// A calendar event for a brewing activity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub event_type: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: String,
    pub notify_minutes_before: Option<i32>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Scalar columns for inserting an event (used by create and batch generation).
#[derive(Debug, Clone)]
pub struct EventWrite {
    pub batch_id: Option<Uuid>,
    pub event_type: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: String,
    pub notify_minutes_before: Option<i32>,
    pub notes: Option<String>,
}

/// Payload for creating an event.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    pub batch_id: Option<Uuid>,
    #[validate(custom(function = "validate_event_type"))]
    pub event_type: String,
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    #[validate(custom(function = "validate_status_opt"))]
    pub status: Option<String>,
    pub notify_minutes_before: Option<i32>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
}

/// Partial-update payload (PATCH). Fields apply only when present.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest {
    pub batch_id: Option<Uuid>,
    #[validate(custom(function = "validate_event_type_opt"))]
    pub event_type: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    #[validate(custom(function = "validate_status_opt"))]
    pub status: Option<String>,
    pub notify_minutes_before: Option<i32>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
}

/// Query parameters for listing events.
#[derive(Debug, Default)]
pub struct ListFilter {
    pub batch_id: Option<Uuid>,
    pub event_type: Option<String>,
    pub status: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

// ---- validators ----

fn one_of(v: &str, allowed: &[&str], code: &'static str) -> Result<(), ValidationError> {
    if allowed.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new(code))
    }
}

const EVENT_TYPES: &[&str] = &[
    "brew_day",
    "dry_hop",
    "fermentation_complete",
    "transfer",
    "package",
    "condition_complete",
    "custom",
];

const STATUSES: &[&str] = &["pending", "completed", "skipped"];

fn validate_event_type(v: &str) -> Result<(), ValidationError> {
    one_of(v, EVENT_TYPES, "invalid_event_type")
}

fn validate_event_type_opt(v: &str) -> Result<(), ValidationError> {
    one_of(v, EVENT_TYPES, "invalid_event_type")
}

fn validate_status_opt(v: &str) -> Result<(), ValidationError> {
    one_of(v, STATUSES, "invalid_status")
}
