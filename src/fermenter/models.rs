//! Fermentation vessel (fermenter) domain types, DTOs, and filters.

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

/// A fermentation vessel a batch can be assigned to.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Fermenter {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub capacity_liters: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Scalar columns for inserting/updating a fermenter.
#[derive(Debug, Clone)]
pub struct FermenterWrite {
    pub name: String,
    pub capacity_liters: Option<f64>,
    pub notes: Option<String>,
}

/// Payload for creating a fermenter.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub capacity_liters: Option<f64>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
}

/// Partial-update payload (PATCH). Fields apply only when present.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(range(exclusive_min = 0.0))]
    pub capacity_liters: Option<f64>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
}

/// Query parameters for listing fermenters.
#[derive(Debug, Default)]
pub struct ListFilter {
    pub name: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}
