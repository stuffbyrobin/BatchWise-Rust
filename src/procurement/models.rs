//! Procurement domain types and DTOs.
//!
//! Port of the Go `internal/procurement` types. `order_date` and
//! `expected_delivery` are `DATE` columns rendered to `YYYY-MM-DD` strings;
//! `created_at`/`updated_at` are `TIMESTAMPTZ`. `quantity` and
//! `received_quantity` are `NUMERIC` (decoded as `f64`); `unit_cost_pence` is
//! money as `i64`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
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

// ---- Supplier ----

/// A vendor of ingredients or materials.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Supplier {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- Purchase Order ----

/// A single line on a purchase order.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PurchaseOrderLine {
    pub id: Uuid,
    pub purchase_order_id: Uuid,
    pub ingredient_type: String,
    pub ingredient_name: String,
    pub quantity: f64,
    pub unit: String,
    pub unit_cost_pence: i64,
    pub unit_cost_currency: String,
    pub received_quantity: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An order raised against a supplier.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PurchaseOrder {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub supplier_id: Uuid,
    pub supplier_name: String,
    pub po_number: String,
    pub status: String,
    pub order_date: String,
    pub expected_delivery: Option<String>,
    pub notes: Option<String>,
    #[sqlx(skip)]
    pub lines: Vec<PurchaseOrderLine>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- request DTOs ----

/// serde helper for "double option" (Go `**T`) nullable-patch fields.
///
/// Absent field → `None` (don't touch). Present field → `Some(inner)` where
/// `inner` is `None` for an explicit JSON `null` (set column to NULL).
fn double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateSupplierRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    #[validate(length(max = 200))]
    pub contact_name: Option<String>,
    #[validate(email, length(max = 320))]
    pub email: Option<String>,
    #[validate(length(max = 50))]
    pub phone: Option<String>,
    #[validate(length(max = 500))]
    pub website: Option<String>,
    #[validate(length(max = 2000))]
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchSupplierRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub contact_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub website: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreatePORequest {
    pub supplier_id: Uuid,
    pub expected_delivery: Option<String>,
    #[validate(length(max = 2000))]
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchPORequest {
    #[validate(custom(function = "validate_patch_status"))]
    pub status: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub expected_delivery: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateLineRequest {
    #[validate(custom(function = "validate_ingredient_type"))]
    pub ingredient_type: String,
    #[validate(length(min = 1, max = 200))]
    pub ingredient_name: String,
    #[validate(range(exclusive_min = 0.0))]
    pub quantity: f64,
    #[validate(length(min = 1, max = 20))]
    pub unit: String,
    #[validate(range(min = 0))]
    pub unit_cost_pence: i64,
    #[serde(default)]
    #[validate(custom(function = "validate_iso_currency_opt"))]
    pub unit_cost_currency: String,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchLineRequest {
    #[validate(custom(function = "validate_ingredient_type_opt"))]
    pub ingredient_type: Option<String>,
    #[validate(length(min = 1, max = 200))]
    pub ingredient_name: Option<String>,
    #[validate(range(exclusive_min = 0.0))]
    pub quantity: Option<f64>,
    #[validate(length(min = 1, max = 20))]
    pub unit: Option<String>,
    #[validate(range(min = 0))]
    pub unit_cost_pence: Option<i64>,
    #[validate(custom(function = "validate_iso_currency_present"))]
    pub unit_cost_currency: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ReceiveRequest {
    #[validate(length(min = 1), nested)]
    pub lines: Vec<ReceiveLine>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ReceiveLine {
    pub line_id: Uuid,
    #[validate(range(min = 0.0))]
    pub received_quantity: f64,
}

// ---- filters ----

#[derive(Debug, Default)]
pub struct SupplierFilter {
    pub search: String,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

#[derive(Debug, Default)]
pub struct POFilter {
    pub supplier_id: Option<Uuid>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

// ---- enums / validators ----

const INGREDIENT_TYPES: [&str; 5] = ["fermentable", "hop", "yeast", "adjunct", "other"];

/// PATCH status allowlist (Go `oneof=sent cancelled received partially_received`).
const PATCH_STATUSES: [&str; 4] = ["sent", "cancelled", "received", "partially_received"];

/// Fixed ISO currency allowlist (matches the Go `iso_currency` validator).
const ISO_CURRENCIES: [&str; 14] = [
    "AED", "AUD", "CAD", "CHF", "CNY", "EUR", "GBP", "HKD", "JPY", "NOK", "NZD", "SEK", "SGD",
    "USD",
];

fn is_iso_currency(v: &str) -> bool {
    v.len() == 3 && v.chars().all(|c| c.is_ascii_uppercase()) && ISO_CURRENCIES.contains(&v)
}

fn validate_ingredient_type(v: &str) -> Result<(), ValidationError> {
    if INGREDIENT_TYPES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_ingredient_type"))
    }
}

fn validate_ingredient_type_opt(v: &str) -> Result<(), ValidationError> {
    validate_ingredient_type(v)
}

fn validate_patch_status(v: &str) -> Result<(), ValidationError> {
    if PATCH_STATUSES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_status"))
    }
}

/// Optional currency: empty string is allowed (defaulted to GBP in the service);
/// otherwise must be a known ISO code.
fn validate_iso_currency_opt(v: &str) -> Result<(), ValidationError> {
    if v.is_empty() || is_iso_currency(v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_currency"))
    }
}

/// Present currency on a PATCH (Go `omitempty,iso_currency`): empty string
/// passes `omitempty`; otherwise must be a known ISO code.
fn validate_iso_currency_present(v: &str) -> Result<(), ValidationError> {
    if v.is_empty() || is_iso_currency(v) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_currency"))
    }
}
