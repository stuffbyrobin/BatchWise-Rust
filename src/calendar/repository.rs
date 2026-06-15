//! Data access for calendar events.
//!
//! Port of the Go `internal/calendar/repository.go`. No `NUMERIC` columns here,
//! so every column is selected as-is; timestamps map to `DateTime<Utc>`.

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Event, EventWrite, ListFilter, Page};

const EVENT_COLS: &str = "id, tenant_id, batch_id, event_type, title, start_time, end_time, \
    status, notify_minutes_before, notes, created_at, updated_at";

fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 {
        20
    } else if page_size > 100 {
        100
    } else {
        page_size
    };
    (page, page_size)
}

/// Inserts an event and returns the created row.
pub async fn insert<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    w: &EventWrite,
) -> Result<Event, sqlx::Error> {
    let sql = format!(
        "INSERT INTO calendar_events (tenant_id, batch_id, event_type, title, start_time, \
         end_time, status, notify_minutes_before, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING {EVENT_COLS}"
    );
    sqlx::query_as::<_, Event>(&sql)
        .bind(tenant_id)
        .bind(w.batch_id)
        .bind(&w.event_type)
        .bind(&w.title)
        .bind(w.start_time)
        .bind(w.end_time)
        .bind(&w.status)
        .bind(w.notify_minutes_before)
        .bind(&w.notes)
        .fetch_one(exec)
        .await
}

/// Updates all scalar columns; returns the new row or `None` if not found.
pub async fn update_full<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    w: &EventWrite,
) -> Result<Option<Event>, sqlx::Error> {
    let sql = format!(
        "UPDATE calendar_events SET batch_id=$3, event_type=$4, title=$5, start_time=$6, \
         end_time=$7, status=$8, notify_minutes_before=$9, notes=$10, updated_at=now() \
         WHERE tenant_id=$1 AND id=$2 RETURNING {EVENT_COLS}"
    );
    sqlx::query_as::<_, Event>(&sql)
        .bind(tenant_id)
        .bind(id)
        .bind(w.batch_id)
        .bind(&w.event_type)
        .bind(&w.title)
        .bind(w.start_time)
        .bind(w.end_time)
        .bind(&w.status)
        .bind(w.notify_minutes_before)
        .bind(&w.notes)
        .fetch_optional(exec)
        .await
}

/// Fetches an event by id, tenant-scoped.
pub async fn select_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Event>, sqlx::Error> {
    let sql = format!("SELECT {EVENT_COLS} FROM calendar_events WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Event>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists events with filters and a pre-validated `order_by` clause.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
    order_by: &str,
) -> Result<Page<Event>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(b) = filter.batch_id {
            qb.push(" AND batch_id = ").push_bind(b);
        }
        if let Some(t) = &filter.event_type {
            qb.push(" AND event_type = ").push_bind(t.clone());
        }
        if let Some(s) = &filter.status {
            qb.push(" AND status = ").push_bind(s.clone());
        }
        if let Some(from) = filter.from {
            qb.push(" AND start_time >= ").push_bind(from);
        }
        if let Some(to) = filter.to {
            qb.push(" AND start_time <= ").push_bind(to);
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM calendar_events");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {EVENT_COLS} FROM calendar_events"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb.build_query_as::<Event>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Deletes an event; returns true if a row was removed.
pub async fn delete_by_id<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM calendar_events WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Counts pending events starting after `from` and up to `to` (used by the dashboard).
pub async fn count_pending_for_range(
    pool: &PgPool,
    tenant_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM calendar_events \
         WHERE tenant_id = $1 AND status = 'pending' AND start_time > $2 AND start_time <= $3",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
}
