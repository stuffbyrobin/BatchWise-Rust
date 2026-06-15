//! Data access for the compliance audit log (append-only).
//!
//! Port of the Go `internal/compliance/audit/repository.go`. `event_data` is a
//! `JSONB` column bound with [`sqlx::types::Json`] and decoded via `#[sqlx(json)]`.
//! Every query is tenant-scoped.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{AuditEvent, AuditEventList, ListFilter};

const COLS: &str = "id, tenant_id, event_type, entity_type, entity_id, actor_user_id, \
    event_data, created_at";

fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 {
        50
    } else if page_size > 200 {
        200
    } else {
        page_size
    };
    (page, page_size)
}

/// Inserts an audit event.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    event_type: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    actor_user_id: Option<Uuid>,
    event_data: &serde_json::Value,
    created_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO compliance_audit_log \
            (id, tenant_id, event_type, entity_type, entity_id, actor_user_id, event_data, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(event_type)
    .bind(entity_type)
    .bind(entity_id)
    .bind(actor_user_id)
    .bind(sqlx::types::Json(event_data))
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetches an audit event by id, tenant-scoped.
pub async fn select_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<AuditEvent>, sqlx::Error> {
    let sql = format!("SELECT {COLS} FROM compliance_audit_log WHERE tenant_id = $1 AND id = $2");
    sqlx::query_as::<_, AuditEvent>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists audit events matching the filter, newest first.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    f: &ListFilter,
) -> Result<AuditEventList, sqlx::Error> {
    let (page, page_size) = clamp_page(f.page, f.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(t) = &f.entity_type {
            qb.push(" AND entity_type = ").push_bind(t.clone());
        }
        if let Some(e) = f.entity_id {
            qb.push(" AND entity_id = ").push_bind(e);
        }
        if let Some(t) = &f.event_type {
            qb.push(" AND event_type = ").push_bind(t.clone());
        }
        if let Some(from) = f.from {
            qb.push(" AND created_at >= ").push_bind(from);
        }
        if let Some(to) = f.to {
            qb.push(" AND created_at <= ").push_bind(to);
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM compliance_audit_log");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {COLS} FROM compliance_audit_log"));
    push_where(&mut qb);
    qb.push(" ORDER BY created_at DESC");
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<AuditEvent>().fetch_all(pool).await?;

    Ok(AuditEventList::new(items, total, page, page_size))
}
