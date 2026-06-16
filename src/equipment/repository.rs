//! Data access for equipment, maintenance schedules, and maintenance events.
//!
//! Port of the Go `internal/equipment/repository.go`. `purchased_at` is a `DATE`
//! column rendered with `to_char` and bound back via `$n::date`. `cost_currency`
//! is `CHAR(3)`, selected with `::text` to avoid blank-padding. Computed columns
//! are aliased to the struct field names so `FromRow` binds them directly. Every
//! query is tenant-scoped.

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{
    Equipment, EventFilter, Filter, MaintenanceDueFilter, MaintenanceDueItem, MaintenanceEvent,
    MaintenanceSchedule, Page, ScheduleFilter,
};

/// SQL expression for a schedule's next-due timestamp, with the given column
/// prefix (`"ms."` in joins, `""` in the bare schedules table query).
fn next_due_expr(prefix: &str) -> String {
    format!(
        "(COALESCE({p}last_performed_at, {p}created_at) + make_interval(days => {p}interval_days))",
        p = prefix
    )
}

/// LATERAL join providing `overdue_count` and `next_due_at` for an equipment row.
fn equipment_computed() -> String {
    let nd = next_due_expr("ms.");
    format!(
        "LEFT JOIN LATERAL ( \
            SELECT \
                COUNT(*) FILTER (WHERE ms.active AND {nd} < now()) AS overdue_count, \
                MIN({nd}) FILTER (WHERE ms.active) AS next_due_at \
            FROM maintenance_schedules ms \
            WHERE ms.equipment_id = e.id \
        ) sc ON true"
    )
}

/// SELECT list for equipment rows (computed columns aliased to struct fields).
fn equipment_cols() -> &'static str {
    "e.id, e.tenant_id, e.name, e.equipment_type, e.serial_number, e.location, e.status, \
     to_char(e.purchased_at, 'YYYY-MM-DD') AS purchased_at, e.notes, \
     COALESCE(sc.overdue_count, 0)::int AS overdue_schedule_count, \
     sc.next_due_at AS next_maintenance_due_at, e.created_at, e.updated_at"
}

/// Schedule SELECT columns plus the three computed fields.
fn schedule_cols() -> String {
    let nd = next_due_expr("");
    format!(
        "id, tenant_id, equipment_id, task_name, interval_days, last_performed_at, active, notes, \
         {nd} AS next_due_at, \
         FLOOR(EXTRACT(EPOCH FROM ({nd} - now())) / 86400)::int AS days_until_due, \
         (active AND {nd} < now()) AS is_overdue, \
         created_at, updated_at"
    )
}

const EVENT_COLS: &str = "id, tenant_id, equipment_id, schedule_id, event_type, performed_at, \
    performed_by, cost_pence, cost_currency::text AS cost_currency, notes, created_at";

/// Matches the Go `normPage`: page<1→1; page_size<1 OR page_size>100 → 20.
fn norm_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if !(1..=100).contains(&page_size) {
        20
    } else {
        page_size
    };
    (page, page_size)
}

fn equipment_order_by(sort: &str) -> &'static str {
    match sort {
        "name" => "e.name ASC",
        "-name" => "e.name DESC",
        "created_at" => "e.created_at ASC",
        "next_maintenance_due_at" => "sc.next_due_at ASC NULLS LAST",
        "-next_maintenance_due_at" => "sc.next_due_at DESC NULLS LAST",
        _ => "e.created_at DESC",
    }
}

fn schedule_order_by(sort: &str) -> &'static str {
    match sort {
        "-next_due_at" => "next_due_at DESC",
        "created_at" => "created_at ASC",
        "-created_at" => "created_at DESC",
        _ => "next_due_at ASC",
    }
}

fn event_order_by(sort: &str) -> &'static str {
    match sort {
        "performed_at" => "performed_at ASC",
        _ => "performed_at DESC",
    }
}

// ---- equipment ----

/// Inserts a piece of equipment and re-reads it (to populate computed fields).
#[allow(clippy::too_many_arguments)]
pub async fn insert_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    name: &str,
    equipment_type: &str,
    serial_number: Option<&str>,
    location: Option<&str>,
    status: &str,
    purchased_at: Option<&str>,
    notes: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO equipment \
         (tenant_id, name, equipment_type, serial_number, location, status, purchased_at, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7::date,$8) RETURNING id",
    )
    .bind(tenant_id)
    .bind(name)
    .bind(equipment_type)
    .bind(serial_number)
    .bind(location)
    .bind(status)
    .bind(purchased_at)
    .bind(notes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Lists equipment with optional status / equipment_type filters. List items
/// leave `lifetime_maintenance_cost_pence` as `None`.
pub async fn select_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &Filter,
) -> Result<Page<Equipment>, sqlx::Error> {
    let (page, page_size) = norm_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE e.tenant_id = ").push_bind(tenant_id);
        if let Some(s) = &filter.status {
            qb.push(" AND e.status = ").push_bind(s.clone());
        }
        if let Some(t) = &filter.equipment_type {
            qb.push(" AND e.equipment_type = ").push_bind(t.clone());
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM equipment e");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = equipment_order_by(&filter.sort);
    let mut qb = QueryBuilder::<Postgres>::new(format!(
        "SELECT {} FROM equipment e {}",
        equipment_cols(),
        equipment_computed()
    ));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by}"));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<Equipment>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a piece of equipment by id, tenant-scoped, populating computed fields
/// and `lifetime_maintenance_cost_pence`.
pub async fn select_equipment_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Equipment>, sqlx::Error> {
    let sql = format!(
        "SELECT {} FROM equipment e {} WHERE e.tenant_id = $1 AND e.id = $2",
        equipment_cols(),
        equipment_computed()
    );
    let row = sqlx::query_as::<_, Equipment>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some(mut e) => {
            let cost: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(cost_pence), 0)::bigint FROM maintenance_events \
                 WHERE equipment_id = $1 AND tenant_id = $2",
            )
            .bind(id)
            .bind(tenant_id)
            .fetch_one(pool)
            .await?;
            e.lifetime_maintenance_cost_pence = Some(cost);
            Ok(Some(e))
        }
        None => Ok(None),
    }
}

/// Updates a piece of equipment's mutable fields; returns true if a row changed.
#[allow(clippy::too_many_arguments)]
pub async fn update_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    name: &str,
    equipment_type: &str,
    serial_number: Option<&str>,
    location: Option<&str>,
    status: &str,
    purchased_at: Option<&str>,
    notes: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE equipment SET name = $3, equipment_type = $4, serial_number = $5, location = $6, \
         status = $7, purchased_at = $8::date, notes = $9, updated_at = now() \
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(name)
    .bind(equipment_type)
    .bind(serial_number)
    .bind(location)
    .bind(status)
    .bind(purchased_at)
    .bind(notes)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a piece of equipment; returns true if a row was removed.
pub async fn delete_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM equipment WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---- schedules ----

/// Inserts a maintenance schedule and returns its id.
#[allow(clippy::too_many_arguments)]
pub async fn insert_schedule(
    pool: &PgPool,
    tenant_id: Uuid,
    equipment_id: Uuid,
    task_name: &str,
    interval_days: i32,
    last_performed_at: Option<chrono::DateTime<chrono::Utc>>,
    active: bool,
    notes: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO maintenance_schedules \
         (tenant_id, equipment_id, task_name, interval_days, last_performed_at, active, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(tenant_id)
    .bind(equipment_id)
    .bind(task_name)
    .bind(interval_days)
    .bind(last_performed_at)
    .bind(active)
    .bind(notes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Lists schedules for a piece of equipment with an optional active filter.
pub async fn select_schedules(
    pool: &PgPool,
    tenant_id: Uuid,
    equipment_id: Uuid,
    filter: &ScheduleFilter,
) -> Result<Page<MaintenanceSchedule>, sqlx::Error> {
    let (page, page_size) = norm_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        qb.push(" AND equipment_id = ").push_bind(equipment_id);
        if let Some(a) = filter.active {
            qb.push(" AND active = ").push_bind(a);
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM maintenance_schedules");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = schedule_order_by(&filter.sort);
    let mut qb = QueryBuilder::<Postgres>::new(format!(
        "SELECT {} FROM maintenance_schedules",
        schedule_cols()
    ));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by}"));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb
        .build_query_as::<MaintenanceSchedule>()
        .fetch_all(pool)
        .await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a schedule by (tenant, equipment, schedule) id.
pub async fn select_schedule_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    equipment_id: Uuid,
    schedule_id: Uuid,
) -> Result<Option<MaintenanceSchedule>, sqlx::Error> {
    let sql = format!(
        "SELECT {} FROM maintenance_schedules \
         WHERE tenant_id = $1 AND equipment_id = $2 AND id = $3",
        schedule_cols()
    );
    sqlx::query_as::<_, MaintenanceSchedule>(&sql)
        .bind(tenant_id)
        .bind(equipment_id)
        .bind(schedule_id)
        .fetch_optional(pool)
        .await
}

/// Looks a schedule up by tenant and id only (any equipment), to distinguish a
/// cross-equipment mismatch (422) from a missing schedule (404).
pub async fn select_schedule_for_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<Option<MaintenanceSchedule>, sqlx::Error> {
    let sql = format!(
        "SELECT {} FROM maintenance_schedules WHERE tenant_id = $1 AND id = $2",
        schedule_cols()
    );
    sqlx::query_as::<_, MaintenanceSchedule>(&sql)
        .bind(tenant_id)
        .bind(schedule_id)
        .fetch_optional(pool)
        .await
}

/// Updates a schedule's mutable fields; returns true if a row changed.
#[allow(clippy::too_many_arguments)]
pub async fn update_schedule(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    task_name: &str,
    interval_days: i32,
    last_performed_at: Option<chrono::DateTime<chrono::Utc>>,
    active: bool,
    notes: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE maintenance_schedules SET task_name = $3, interval_days = $4, \
         last_performed_at = $5, active = $6, notes = $7, updated_at = now() \
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(task_name)
    .bind(interval_days)
    .bind(last_performed_at)
    .bind(active)
    .bind(notes)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a schedule by (tenant, schedule) id; returns true if a row was removed.
pub async fn delete_schedule(
    pool: &PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM maintenance_schedules WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(schedule_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Sets `last_performed_at` to `performed_at` only when it moves the schedule
/// forward (current value is null or earlier). No error on no-op.
pub async fn advance_schedule_last_performed(
    pool: &PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
    performed_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE maintenance_schedules SET last_performed_at = $3, updated_at = now() \
         WHERE tenant_id = $1 AND id = $2 \
         AND (last_performed_at IS NULL OR last_performed_at < $3)",
    )
    .bind(tenant_id)
    .bind(schedule_id)
    .bind(performed_at)
    .execute(pool)
    .await?;
    Ok(())
}

// ---- events ----

/// Inserts a maintenance event and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_event(
    pool: &PgPool,
    tenant_id: Uuid,
    equipment_id: Uuid,
    schedule_id: Option<Uuid>,
    event_type: &str,
    performed_at: chrono::DateTime<chrono::Utc>,
    performed_by: Option<&str>,
    cost_pence: Option<i64>,
    cost_currency: &str,
    notes: Option<&str>,
) -> Result<MaintenanceEvent, sqlx::Error> {
    let sql = format!(
        "INSERT INTO maintenance_events \
         (tenant_id, equipment_id, schedule_id, event_type, performed_at, performed_by, \
          cost_pence, cost_currency, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING {EVENT_COLS}"
    );
    sqlx::query_as::<_, MaintenanceEvent>(&sql)
        .bind(tenant_id)
        .bind(equipment_id)
        .bind(schedule_id)
        .bind(event_type)
        .bind(performed_at)
        .bind(performed_by)
        .bind(cost_pence)
        .bind(cost_currency)
        .bind(notes)
        .fetch_one(pool)
        .await
}

/// Lists events for a piece of equipment with optional event_type / schedule_id filters.
pub async fn select_events(
    pool: &PgPool,
    tenant_id: Uuid,
    equipment_id: Uuid,
    filter: &EventFilter,
) -> Result<Page<MaintenanceEvent>, sqlx::Error> {
    let (page, page_size) = norm_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        qb.push(" AND equipment_id = ").push_bind(equipment_id);
        if let Some(et) = &filter.event_type {
            qb.push(" AND event_type = ").push_bind(et.clone());
        }
        if let Some(sid) = filter.schedule_id {
            qb.push(" AND schedule_id = ").push_bind(sid);
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM maintenance_events");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = event_order_by(&filter.sort);
    let mut qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {EVENT_COLS} FROM maintenance_events"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by}"));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb
        .build_query_as::<MaintenanceEvent>()
        .fetch_all(pool)
        .await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches an event by (tenant, equipment, event) id.
pub async fn select_event_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    equipment_id: Uuid,
    event_id: Uuid,
) -> Result<Option<MaintenanceEvent>, sqlx::Error> {
    let sql = format!(
        "SELECT {EVENT_COLS} FROM maintenance_events \
         WHERE tenant_id = $1 AND equipment_id = $2 AND id = $3"
    );
    sqlx::query_as::<_, MaintenanceEvent>(&sql)
        .bind(tenant_id)
        .bind(equipment_id)
        .bind(event_id)
        .fetch_optional(pool)
        .await
}

/// Deletes an event by (tenant, event) id; returns true if a row was removed.
pub async fn delete_event(
    pool: &PgPool,
    tenant_id: Uuid,
    event_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM maintenance_events WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(event_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---- maintenance due feed ----

/// Lists schedules that are overdue or due within a window, denormalised with
/// their equipment details.
pub async fn select_maintenance_due(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &MaintenanceDueFilter,
) -> Result<Page<MaintenanceDueItem>, sqlx::Error> {
    let (page, page_size) = norm_page(filter.page, filter.page_size);
    let window_days = if filter.window_days < 0 {
        0
    } else {
        filter.window_days
    };
    let nd = next_due_expr("ms.");

    // Builds the FROM + WHERE shared by count and select. The window_days bind
    // is only pushed when not overdue-only (matches the Go conditional bind).
    let push_from_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(format!(
            "FROM maintenance_schedules ms \
             JOIN equipment e ON e.id = ms.equipment_id AND e.tenant_id = ms.tenant_id \
             CROSS JOIN LATERAL (SELECT {nd} AS next_due_at) nd \
             WHERE ms.tenant_id = "
        ));
        qb.push_bind(tenant_id);
        qb.push(" AND ms.active");
        if filter.overdue_only {
            qb.push(" AND nd.next_due_at < now()");
        } else {
            qb.push(" AND nd.next_due_at <= now() + make_interval(days => (");
            qb.push_bind(window_days);
            qb.push(")::int)");
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) ");
    push_from_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT ms.id AS schedule_id, ms.equipment_id, e.name AS equipment_name, \
         e.equipment_type AS equipment_type, ms.task_name, ms.interval_days, \
         ms.last_performed_at, nd.next_due_at, \
         FLOOR(EXTRACT(EPOCH FROM (nd.next_due_at - now())) / 86400)::int AS days_until_due, \
         (nd.next_due_at < now()) AS is_overdue ",
    );
    push_from_where(&mut qb);
    qb.push(" ORDER BY nd.next_due_at ASC");
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb
        .build_query_as::<MaintenanceDueItem>()
        .fetch_all(pool)
        .await?;
    Ok(Page::new(items, total, page, page_size))
}
