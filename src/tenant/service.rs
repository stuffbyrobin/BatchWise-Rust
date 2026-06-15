//! Tenant business logic.
//!
//! Port of the Go `internal/tenant/service.go`. Ownership checks read the user
//! via the auth repository (same crate, so no adapter is needed).

use sqlx::PgPool;
use uuid::Uuid;

use super::models::{Tenant, UpdateRequest};
use super::repository;
use crate::auth::repository as auth_repo;
use crate::platform::errors::ApiError;

/// Returns the current tenant.
pub async fn get_current(pool: &PgPool, tenant_id: Uuid) -> Result<Tenant, ApiError> {
    repository::get_by_id(pool, tenant_id)
        .await?
        .ok_or_else(|| ApiError::not_found("tenant"))
}

/// Updates the current tenant. Only the tenant owner may do so.
pub async fn update(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    req: UpdateRequest,
) -> Result<Tenant, ApiError> {
    let user = auth_repo::get_user_by_id(pool, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("user"))?;
    if !user.is_owner {
        return Err(ApiError::forbidden(
            "only the tenant owner can update tenant settings",
        ));
    }

    let mut tn = repository::get_by_id(pool, tenant_id)
        .await?
        .ok_or_else(|| ApiError::not_found("tenant"))?;

    if let Some(name) = &req.tenant_name {
        if name != &tn.tenant_name {
            if repository::get_by_name(pool, name).await?.is_some() {
                return Err(ApiError::conflict("tenant_name", "already taken"));
            }
            tn.tenant_name = name.clone();
        }
    }
    if let Some(country) = req.country {
        tn.country = country;
    }
    if req.region.is_some() {
        tn.region = req.region;
    }
    if let Some(address) = req.address {
        tn.address = address;
    }
    if req.next_batch_number.is_some() {
        tn.next_batch_number = req.next_batch_number;
    }
    if req.next_order_number.is_some() {
        tn.next_order_number = req.next_order_number;
    }
    if let Some(ibu) = req.ibu_method {
        tn.ibu_method = ibu;
    }
    if let Some(sbr) = req.sbr_annual_production_hl_pa {
        tn.sbr_annual_production_hl_pa = sbr;
    }

    repository::update(
        pool,
        tenant_id,
        &tn.tenant_name,
        &tn.country,
        tn.region.as_deref(),
        &tn.address,
        &tn.feature_flags,
        tn.next_batch_number,
        tn.next_order_number,
        &tn.ibu_method,
        tn.sbr_annual_production_hl_pa,
    )
    .await?
    .ok_or_else(|| ApiError::not_found("tenant"))
}
