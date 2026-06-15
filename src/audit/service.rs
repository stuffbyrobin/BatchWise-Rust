//! Compliance-audit business logic.
//!
//! Port of the Go `internal/compliance/audit/service.go`. [`write`] records an
//! audit event fire-and-forget: any error is logged and swallowed, never
//! propagated, so an audit failure can't break the business operation that
//! triggered it.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{AuditEvent, AuditEventList, ListFilter, WriteRequest};
use super::repository as repo;
use crate::platform::errors::ApiError;

/// Records an audit event. Errors are logged and swallowed — never returned.
pub async fn write(pool: &PgPool, req: WriteRequest) {
    let id = Uuid::new_v4();
    if let Err(e) = repo::insert(
        pool,
        id,
        req.tenant_id,
        req.event_type,
        req.entity_type,
        req.entity_id,
        req.actor_user_id,
        &req.event_data,
        Utc::now(),
    )
    .await
    {
        tracing::error!(event_type = req.event_type, error = %e, "audit: insert failed");
    }
}

/// Lists audit events matching the filter.
pub async fn list(
    pool: &PgPool,
    tenant_id: Uuid,
    f: ListFilter,
) -> Result<AuditEventList, ApiError> {
    Ok(repo::select_list(pool, tenant_id, &f).await?)
}

/// Returns a single audit event, or a not-found error.
pub async fn get(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<AuditEvent, ApiError> {
    repo::select_by_id(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("audit_event"))
}
