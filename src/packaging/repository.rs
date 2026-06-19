//! Data access for packaging runs and distribution movements.
//!
//! Port of the Go `internal/packaging/repository.go`. `packaged_at` and
//! `best_before_date` are `DATE` columns rendered with `to_char` and bound back
//! via `::date`. `stock_remaining` is computed from movements (outbound types
//! reduce stock, `return` adds it back).

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{
    DistributionMovement, ListMovementsFilter, ListPackagingRunsFilter, PackagingRun, Page,
};

/// The packaging-run columns plus the derived `stock_remaining`, for queries
/// that LEFT JOIN distribution_movements and GROUP BY pr.id.
const RUN_COLS: &str = "pr.id, pr.tenant_id, pr.batch_id, pr.format, pr.unit_volume_ml, \
    pr.quantity, pr.lot_number, to_char(pr.packaged_at, 'YYYY-MM-DD') AS packaged_at, \
    to_char(pr.best_before_date, 'YYYY-MM-DD') AS best_before_date, pr.notes, \
    pr.quantity \
        - COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type IN \
            ('sale','taproom_transfer','internal_transfer','sample','disposal')), 0) \
        + COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type = 'return'), 0) \
        AS stock_remaining, \
    pr.created_at, pr.updated_at";

const RUN_FROM: &str =
    "FROM packaging_runs pr LEFT JOIN distribution_movements dm ON dm.packaging_run_id = pr.id";

const MOV_COLS: &str = "dm.id, dm.tenant_id, dm.packaging_run_id, dm.movement_type, dm.quantity, \
    dm.from_location, dm.to_location, dm.order_id, dm.reference, dm.notes, dm.moved_at, \
    dm.created_at";

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

// ---- packaging runs ----

/// Inserts a packaging run and returns it (a fresh run has `stock_remaining = quantity`).
#[allow(clippy::too_many_arguments)]
pub async fn insert_run(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
    format: &str,
    unit_volume_ml: i32,
    quantity: i32,
    lot_number: &str,
    packaged_at: &str,
    best_before_date: Option<&str>,
    notes: Option<&str>,
) -> Result<PackagingRun, sqlx::Error> {
    let sql = "INSERT INTO packaging_runs (tenant_id, batch_id, format, unit_volume_ml, quantity, \
        lot_number, packaged_at, best_before_date, notes) \
        VALUES ($1,$2,$3,$4,$5,$6,$7::date,$8::date,$9) \
        RETURNING id, tenant_id, batch_id, format, unit_volume_ml, quantity, lot_number, \
            to_char(packaged_at, 'YYYY-MM-DD') AS packaged_at, \
            to_char(best_before_date, 'YYYY-MM-DD') AS best_before_date, notes, \
            quantity::bigint AS stock_remaining, created_at, updated_at";
    sqlx::query_as::<_, PackagingRun>(sql)
        .bind(tenant_id)
        .bind(batch_id)
        .bind(format)
        .bind(unit_volume_ml)
        .bind(quantity)
        .bind(lot_number)
        .bind(packaged_at)
        .bind(best_before_date)
        .bind(notes)
        .fetch_one(pool)
        .await
}

/// Fetches a packaging run by id, tenant-scoped.
pub async fn select_run_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<PackagingRun>, sqlx::Error> {
    let sql = format!(
        "SELECT {RUN_COLS} {RUN_FROM} WHERE pr.id = $1 AND pr.tenant_id = $2 GROUP BY pr.id"
    );
    sqlx::query_as::<_, PackagingRun>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Updates the mutable fields of a packaging run (best_before_date, notes).
pub async fn update_run(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    best_before_date: Option<&str>,
    notes: Option<&str>,
) -> Result<Option<PackagingRun>, sqlx::Error> {
    let sql = "WITH upd AS ( \
            UPDATE packaging_runs \
            SET best_before_date = $3::date, notes = $4, updated_at = now() \
            WHERE id = $1 AND tenant_id = $2 \
            RETURNING id, tenant_id, batch_id, format, unit_volume_ml, quantity, lot_number, \
                packaged_at, best_before_date, notes, created_at, updated_at \
        ) \
        SELECT upd.id, upd.tenant_id, upd.batch_id, upd.format, upd.unit_volume_ml, upd.quantity, \
            upd.lot_number, to_char(upd.packaged_at, 'YYYY-MM-DD') AS packaged_at, \
            to_char(upd.best_before_date, 'YYYY-MM-DD') AS best_before_date, upd.notes, \
            (SELECT upd.quantity \
                - COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type IN \
                    ('sale','taproom_transfer','internal_transfer','sample','disposal')), 0) \
                + COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type = 'return'), 0) \
                FROM distribution_movements dm WHERE dm.packaging_run_id = upd.id) AS stock_remaining, \
            upd.created_at, upd.updated_at \
        FROM upd";
    sqlx::query_as::<_, PackagingRun>(sql)
        .bind(id)
        .bind(tenant_id)
        .bind(best_before_date)
        .bind(notes)
        .fetch_optional(pool)
        .await
}

/// Deletes a packaging run; returns true if a row was removed.
pub async fn delete_run(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM packaging_runs WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// True if the packaging run has any distribution movements.
pub async fn has_movements(
    pool: &PgPool,
    tenant_id: Uuid,
    packaging_run_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM distribution_movements \
         WHERE packaging_run_id = $1 AND tenant_id = $2)",
    )
    .bind(packaging_run_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await
}

/// Returns remaining stock for a run, or `None` if the run does not exist.
pub async fn stock_remaining(
    pool: &PgPool,
    tenant_id: Uuid,
    packaging_run_id: Uuid,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT pr.quantity \
            - COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type IN \
                ('sale','taproom_transfer','internal_transfer','sample','disposal')), 0) \
            + COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type = 'return'), 0) \
         FROM packaging_runs pr \
         LEFT JOIN distribution_movements dm ON dm.packaging_run_id = pr.id \
         WHERE pr.id = $1 AND pr.tenant_id = $2 \
         GROUP BY pr.quantity",
    )
    .bind(packaging_run_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
}

/// Lists packaging runs with filters.
/// Safe `ORDER BY` for packaging runs (alias `pr`); default `-packaged_at`.
/// `pr.created_at DESC` is kept as a stable tiebreaker.
fn build_run_sort(sort: &str) -> String {
    let spec = if sort.is_empty() {
        "-packaged_at"
    } else {
        sort
    };
    let desc = spec.starts_with('-');
    let col = match spec.trim_start_matches('-') {
        "lot_number" => "pr.lot_number",
        "format" => "pr.format",
        "unit_volume_ml" => "pr.unit_volume_ml",
        "quantity" => "pr.quantity",
        "packaged_at" => "pr.packaged_at",
        "best_before_date" => "pr.best_before_date",
        _ => "pr.packaged_at",
    };
    format!(
        "{col} {}, pr.created_at DESC",
        if desc { "DESC" } else { "ASC" }
    )
}

/// Safe `ORDER BY` for distribution movements (alias `dm`); default `-moved_at`.
fn build_movement_sort(sort: &str) -> String {
    let spec = if sort.is_empty() { "-moved_at" } else { sort };
    let desc = spec.starts_with('-');
    let col = match spec.trim_start_matches('-') {
        "movement_type" => "dm.movement_type",
        "quantity" => "dm.quantity",
        "from_location" => "dm.from_location",
        "to_location" => "dm.to_location",
        "moved_at" => "dm.moved_at",
        "reference" => "dm.reference",
        _ => "dm.moved_at",
    };
    format!(
        "{col} {}, dm.created_at DESC",
        if desc { "DESC" } else { "ASC" }
    )
}

pub async fn select_runs(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListPackagingRunsFilter,
) -> Result<Page<PackagingRun>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE pr.tenant_id = ").push_bind(tenant_id);
        if let Some(b) = filter.batch_id {
            qb.push(" AND pr.batch_id = ").push_bind(b);
        }
        if let Some(f) = &filter.format {
            qb.push(" AND pr.format = ").push_bind(f.clone());
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM packaging_runs pr");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {RUN_COLS} {RUN_FROM}"));
    push_where(&mut qb);
    qb.push(format!(
        " GROUP BY pr.id ORDER BY {}",
        build_run_sort(&filter.sort)
    ));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<PackagingRun>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

// ---- distribution movements ----

/// Inserts a distribution movement and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_movement(
    pool: &PgPool,
    tenant_id: Uuid,
    packaging_run_id: Uuid,
    movement_type: &str,
    quantity: i32,
    from_location: &str,
    to_location: &str,
    order_id: Option<Uuid>,
    reference: Option<&str>,
    notes: Option<&str>,
    moved_at: DateTime<Utc>,
) -> Result<DistributionMovement, sqlx::Error> {
    let sql = "INSERT INTO distribution_movements (tenant_id, packaging_run_id, movement_type, \
        quantity, from_location, to_location, order_id, reference, notes, moved_at) \
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
        RETURNING id, tenant_id, packaging_run_id, movement_type, quantity, from_location, \
            to_location, order_id, reference, notes, moved_at, created_at";
    sqlx::query_as::<_, DistributionMovement>(sql)
        .bind(tenant_id)
        .bind(packaging_run_id)
        .bind(movement_type)
        .bind(quantity)
        .bind(from_location)
        .bind(to_location)
        .bind(order_id)
        .bind(reference)
        .bind(notes)
        .bind(moved_at)
        .fetch_one(pool)
        .await
}

/// Fetches a movement by id, tenant-scoped.
pub async fn select_movement_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<DistributionMovement>, sqlx::Error> {
    let sql = format!(
        "SELECT {MOV_COLS} FROM distribution_movements dm WHERE dm.id = $1 AND dm.tenant_id = $2"
    );
    sqlx::query_as::<_, DistributionMovement>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Deletes a movement; returns true if a row was removed.
pub async fn delete_movement(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM distribution_movements WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Lists distribution movements with filters.
pub async fn select_movements(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListMovementsFilter,
) -> Result<Page<DistributionMovement>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE dm.tenant_id = ").push_bind(tenant_id);
        if let Some(r) = filter.packaging_run_id {
            qb.push(" AND dm.packaging_run_id = ").push_bind(r);
        }
        if let Some(o) = filter.order_id {
            qb.push(" AND dm.order_id = ").push_bind(o);
        }
        if let Some(t) = &filter.movement_type {
            qb.push(" AND dm.movement_type = ").push_bind(t.clone());
        }
    };
    let mut count_qb =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM distribution_movements dm");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {MOV_COLS} FROM distribution_movements dm"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {}", build_movement_sort(&filter.sort)));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb
        .build_query_as::<DistributionMovement>()
        .fetch_all(pool)
        .await?;
    Ok(Page::new(items, total, page, page_size))
}
