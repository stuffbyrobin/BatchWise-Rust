//! Sales domain types and DTOs.
//!
//! Port of the Go `internal/sales` types. `NUMERIC` columns are selected as
//! `float8`; `DATE` columns are rendered to `YYYY-MM-DD` strings; money is held
//! as `i64` pence.

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

// ---- domain models ----

/// A customer.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub postcode: Option<String>,
    pub country: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An order line item.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct OrderItem {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub order_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub product_name: String,
    pub volume_liters: f64,
    pub unit_price_pence: i64,
    pub quantity: i32,
    pub total_price_pence: i64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// An order. `customer_name` and `total_price_pence` are computed at query time.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Order {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub order_number: String,
    pub status: String,
    pub order_date: String,
    pub fulfillment_date: Option<String>,
    pub notes: Option<String>,
    pub total_price_pence: i64,
    #[sqlx(skip)]
    pub items: Vec<OrderItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A crystallised duty event.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DutyEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub order_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub event_type: String,
    pub volume_liters: f64,
    pub abv_pct: f64,
    pub duty_pence: i64,
    pub jurisdiction: String,
    pub crystallised_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ---- filters ----

#[derive(Debug, Default)]
pub struct CustomerFilter {
    pub q: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

#[derive(Debug, Default)]
pub struct OrderFilter {
    pub customer_id: Option<Uuid>,
    pub status: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

#[derive(Debug, Default)]
pub struct DutyEventFilter {
    pub order_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

// ---- request DTOs ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateCustomerRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub postcode: Option<String>,
    #[validate(length(min = 2, max = 2))]
    pub country: String,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchCustomerRequest {
    pub name: Option<String>,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateOrderRequest {
    pub customer_id: Uuid,
    pub order_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchOrderRequest {
    pub order_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateItemRequest {
    pub batch_id: Option<Uuid>,
    #[validate(length(min = 1, max = 255))]
    pub product_name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub volume_liters: f64,
    #[validate(range(min = 0))]
    pub unit_price_pence: i64,
    pub quantity: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FulfillOrderRequest {
    pub fulfillment_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CancelOrderRequest {
    pub notes: Option<String>,
}

// ---- validators (unused for now; reserved for enum-style fields) ----

#[allow(dead_code)]
fn validate_event_type(v: &str) -> Result<(), ValidationError> {
    const EVENT_TYPES: [&str; 4] = ["sale", "sample", "waste", "export"];
    if EVENT_TYPES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_event_type"))
    }
}
