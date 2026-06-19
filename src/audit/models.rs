//! Compliance-audit domain types and DTOs.
//!
//! Port of the Go `internal/compliance/audit` types. `event_data` is a `JSONB`
//! column carried as a raw [`serde_json::Value`]; `created_at` is `TIMESTAMPTZ`.
//! The log is append-only — there is no `updated_at`.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

// ---- audit event type constants ----

pub const EVENT_LABEL_CREATED: &str = "label_record.created";
pub const EVENT_LABEL_UPDATED: &str = "label_record.updated";
pub const EVENT_LABEL_APPROVED: &str = "label_record.approved";
pub const EVENT_LABEL_DELETED: &str = "label_record.deleted";

pub const EVENT_DUTY_COMPILED: &str = "duty_return.compiled";
pub const EVENT_DUTY_SUBMITTED: &str = "duty_return.submitted";

pub const EVENT_ALLERGEN_COMPUTED: &str = "allergen_result.computed";

pub const EVENT_PACKAGING_RUN_CREATED: &str = "packaging_run.created";
pub const EVENT_PACKAGING_RUN_DELETED: &str = "packaging_run.deleted";

pub const EVENT_MOVEMENT_CREATED: &str = "distribution_movement.created";
pub const EVENT_MOVEMENT_DELETED: &str = "distribution_movement.deleted";

pub const EVENT_RECALL_QUERIED: &str = "recall.queried";

/// A single compliance audit log entry.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AuditEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub actor_user_id: Option<Uuid>,
    #[sqlx(json)]
    pub event_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// A paginated list of audit events.
#[derive(Debug, Serialize)]
pub struct AuditEventList {
    pub items: Vec<AuditEvent>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl AuditEventList {
    pub fn new(items: Vec<AuditEvent>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if total > 0 && page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        AuditEventList {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

/// Input to [`super::service::write`]. `event_data` is marshalled to `JSONB`.
#[derive(Debug)]
pub struct WriteRequest {
    pub tenant_id: Uuid,
    pub event_type: &'static str,
    pub entity_type: &'static str,
    pub entity_id: Option<Uuid>,
    pub actor_user_id: Option<Uuid>,
    pub event_data: serde_json::Value,
}

/// Filters for [`super::service::list`].
#[derive(Debug, Default)]
pub struct ListFilter {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub event_type: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub sort: String,
    pub page: i64,
    pub page_size: i64,
}
