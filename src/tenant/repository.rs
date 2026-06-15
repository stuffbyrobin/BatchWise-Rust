//! Data access for tenants.
//!
//! Port of the Go `internal/tenant/repository.go`. `feature_flags` is JSONB
//! (decoded via `#[sqlx(json)]`); `sbr_annual_production_hl_pa` is `NUMERIC`,
//! selected as `float8` so it maps to `f64`.

use std::collections::HashMap;

use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use super::models::Tenant;

const TENANT_COLS: &str = "id, tenant_name, tier, country, region, address, feature_flags, \
    next_batch_number, next_order_number, ibu_method, \
    sbr_annual_production_hl_pa::float8 AS sbr_annual_production_hl_pa, created_at, updated_at";

/// Fetches a tenant by id.
pub async fn get_by_id(pool: &PgPool, tenant_id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
    let sql = format!("SELECT {TENANT_COLS} FROM tenants WHERE id = $1");
    sqlx::query_as::<_, Tenant>(&sql)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Fetches a tenant by name.
pub async fn get_by_name(pool: &PgPool, name: &str) -> Result<Option<Tenant>, sqlx::Error> {
    let sql = format!("SELECT {TENANT_COLS} FROM tenants WHERE tenant_name = $1");
    sqlx::query_as::<_, Tenant>(&sql)
        .bind(name)
        .fetch_optional(pool)
        .await
}

/// Inserts a tenant, returning its new id. Executor-generic so it can run
/// inside the register transaction.
pub async fn insert<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_name: &str,
    tier: &str,
    country: &str,
    region: Option<&str>,
    feature_flags: &HashMap<String, bool>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO tenants (tenant_name, tier, country, region, feature_flags) \
         VALUES ($1, $2, $3, $4, $5::jsonb) RETURNING id",
    )
    .bind(tenant_name)
    .bind(tier)
    .bind(country)
    .bind(region)
    .bind(sqlx::types::Json(feature_flags))
    .fetch_one(exec)
    .await
}

/// Persists the full set of updatable tenant columns and returns the fresh row.
#[allow(clippy::too_many_arguments)]
pub async fn update(
    pool: &PgPool,
    tenant_id: Uuid,
    tenant_name: &str,
    country: &str,
    region: Option<&str>,
    address: &str,
    feature_flags: &HashMap<String, bool>,
    next_batch_number: Option<i32>,
    next_order_number: Option<i32>,
    ibu_method: &str,
    sbr_annual_production_hl_pa: f64,
) -> Result<Option<Tenant>, sqlx::Error> {
    sqlx::query(
        "UPDATE tenants SET tenant_name=$1, country=$2, region=$3, address=$4, \
         feature_flags=$5::jsonb, next_batch_number=$6, next_order_number=$7, \
         ibu_method=$8, sbr_annual_production_hl_pa=$9, updated_at=now() WHERE id=$10",
    )
    .bind(tenant_name)
    .bind(country)
    .bind(region)
    .bind(address)
    .bind(sqlx::types::Json(feature_flags))
    .bind(next_batch_number)
    .bind(next_order_number)
    .bind(ibu_method)
    .bind(sbr_annual_production_hl_pa)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    get_by_id(pool, tenant_id).await
}
