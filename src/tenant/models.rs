//! Tenant domain types and DTOs.
//!
//! Port of the Go `internal/tenant/models.go`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Full tenant record.
#[derive(Debug, Clone, FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub tenant_name: String,
    pub tier: String,
    pub country: String,
    pub region: Option<String>,
    pub address: String,
    #[sqlx(json)]
    pub feature_flags: HashMap<String, bool>,
    pub next_batch_number: Option<i32>,
    pub next_order_number: Option<i32>,
    pub ibu_method: String,
    pub sbr_annual_production_hl_pa: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Body for `PATCH /tenants/current`.
#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest {
    pub tenant_name: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub address: Option<String>,
    pub next_batch_number: Option<i32>,
    pub next_order_number: Option<i32>,
    pub ibu_method: Option<String>,
    #[validate(range(min = 0.0))]
    pub sbr_annual_production_hl_pa: Option<f64>,
}

impl UpdateRequest {
    /// True when no field was provided (mirrors the Go "at least one field" guard).
    pub fn is_empty(&self) -> bool {
        self.tenant_name.is_none()
            && self.country.is_none()
            && self.region.is_none()
            && self.address.is_none()
            && self.next_batch_number.is_none()
            && self.next_order_number.is_none()
            && self.ibu_method.is_none()
            && self.sbr_annual_production_hl_pa.is_none()
    }
}

/// JSON response shape for tenant endpoints.
#[derive(Debug, Serialize)]
pub struct Response {
    pub id: Uuid,
    pub tenant_name: String,
    pub tier: String,
    pub country: String,
    pub region: Option<String>,
    pub address: String,
    pub feature_flags: HashMap<String, bool>,
    pub next_batch_number: Option<i32>,
    pub next_order_number: Option<i32>,
    pub ibu_method: String,
    pub sbr_annual_production_hl_pa: f64,
    pub created_at: DateTime<Utc>,
}

impl From<Tenant> for Response {
    fn from(t: Tenant) -> Self {
        Response {
            id: t.id,
            tenant_name: t.tenant_name,
            tier: t.tier,
            country: t.country,
            region: t.region,
            address: t.address,
            feature_flags: t.feature_flags,
            next_batch_number: t.next_batch_number,
            next_order_number: t.next_order_number,
            ibu_method: t.ibu_method,
            sbr_annual_production_hl_pa: t.sbr_annual_production_hl_pa,
            created_at: t.created_at,
        }
    }
}
