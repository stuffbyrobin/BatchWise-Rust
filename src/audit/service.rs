//! Compliance-audit business logic.
//!
//! Port of the Go `internal/compliance/audit/service.go`. [`write`] records an
//! audit event fire-and-forget: any error is logged and swallowed, never
//! propagated, so an audit failure can't break the business operation that
//! triggered it.

use chrono::Utc;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use super::models::{AuditEvent, AuditEventList, ListFilter, WriteRequest};
use super::repository as repo;
use crate::platform::errors::ApiError;

/// Records an audit event on the caller's executor and returns any error.
///
/// Pass the transaction that makes the audited change, so the change and its
/// audit row commit together and a failed insert rolls the change back: the
/// compliance log must not silently miss events. Read-only operations (such as a
/// recall query) may pass the pool.
pub async fn write<'e, E: PgExecutor<'e>>(exec: E, req: WriteRequest) -> Result<(), ApiError> {
    repo::insert(
        exec,
        Uuid::new_v4(),
        req.tenant_id,
        req.event_type,
        req.entity_type,
        req.entity_id,
        req.actor_user_id,
        &req.event_data,
        Utc::now(),
    )
    .await?;
    Ok(())
}

/// Lists audit events matching the filter.
pub async fn list(
    pool: &PgPool,
    tenant_id: Uuid,
    f: ListFilter,
) -> Result<AuditEventList, ApiError> {
    repo::select_list(pool, tenant_id, &f).await
}

/// Returns a single audit event, or a not-found error.
pub async fn get(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<AuditEvent, ApiError> {
    repo::select_by_id(pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("audit_event"))
}
