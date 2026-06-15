//! Data access for traceability queries.
//!
//! Port of the Go `internal/traceability/repository.go`. Every query is
//! tenant-scoped. `DATE` columns are rendered with `to_char(..., 'YYYY-MM-DD')`
//! so they arrive as `String`; `moved_at` (`TIMESTAMPTZ`) is a `DateTime<Utc>`.
//! `stock_remaining` is computed from `distribution_movements` exactly as in the
//! Go SQL (sale/taproom_transfer/internal_transfer/sample/disposal subtract,
//! return adds). Missing root rows map to `ApiError::not_found`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    AffectedCustomer, BatchSummary, IngredientLotSummary, MovementSummary, PackagingRunSummary,
};
use crate::platform::errors::ApiError;

/// Row shape for an ingredient-lot summary query (id, lot_number, name, type,
/// supplier, best_before_date, allergens).
type LotRow = (
    Uuid,
    String,
    String,
    String,
    String,
    Option<String>,
    Vec<String>,
);

/// Row shape for a distribution-movement query (id, run_id, type, quantity,
/// to_location, order_id, order_number, moved_at).
type MovementRow = (
    Uuid,
    Uuid,
    String,
    i32,
    String,
    Option<Uuid>,
    String,
    DateTime<Utc>,
);

/// Fetches an ingredient lot by its lot number, tenant-scoped.
/// Returns `not_found("ingredient_lot")` when absent.
pub async fn ingredient_lot_by_number(
    pool: &PgPool,
    tenant_id: Uuid,
    lot_number: &str,
) -> Result<IngredientLotSummary, ApiError> {
    let row: Option<LotRow> = sqlx::query_as(
        "SELECT id, lot_number, name, type, COALESCE(supplier, ''), \
             to_char(best_before_date, 'YYYY-MM-DD'), allergens \
             FROM ingredients \
             WHERE tenant_id = $1 AND lot_number = $2",
    )
    .bind(tenant_id)
    .bind(lot_number)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((lot_id, lot_number, name, r#type, supplier, best_before_date, allergens)) => {
            Ok(IngredientLotSummary {
                lot_id,
                lot_number,
                name,
                r#type,
                supplier,
                best_before_date,
                allergens,
            })
        }
        None => Err(ApiError::not_found("ingredient_lot")),
    }
}

/// Returns the distinct batches that consumed a given ingredient lot,
/// newest brew date first.
pub async fn batches_by_ingredient_lot(
    pool: &PgPool,
    tenant_id: Uuid,
    ingredient_id: Uuid,
) -> Result<Vec<BatchSummary>, ApiError> {
    let rows: Vec<(Uuid, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT DISTINCT b.id, b.batch_number, b.name, b.status, \
            to_char(b.brew_date, 'YYYY-MM-DD') \
         FROM batch_ingredients bi \
         JOIN batches b ON b.id = bi.batch_id \
         WHERE bi.ingredient_id = $1 \
           AND b.tenant_id = $2 \
         ORDER BY to_char(b.brew_date, 'YYYY-MM-DD') DESC NULLS LAST",
    )
    .bind(ingredient_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(batch_id, batch_number, name, status, brew_date)| BatchSummary {
                batch_id,
                batch_number,
                name,
                status,
                brew_date,
            },
        )
        .collect())
}

const RUN_SELECT: &str = "SELECT pr.id, pr.batch_id, pr.lot_number, pr.format, pr.unit_volume_ml, \
        pr.quantity, \
        (pr.quantity \
            - COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type IN \
                ('sale','taproom_transfer','internal_transfer','sample','disposal')), 0) \
            + COALESCE(SUM(dm.quantity) FILTER (WHERE dm.movement_type = 'return'), 0))::int \
            AS stock_remaining, \
        to_char(pr.packaged_at, 'YYYY-MM-DD') AS packaged_at, \
        to_char(pr.best_before_date, 'YYYY-MM-DD') AS best_before_date \
     FROM packaging_runs pr \
     LEFT JOIN distribution_movements dm ON dm.packaging_run_id = pr.id";

type RunRow = (
    Uuid,
    Uuid,
    String,
    String,
    i32,
    i32,
    i32,
    String,
    Option<String>,
);

fn run_from_row(r: RunRow) -> PackagingRunSummary {
    let (
        run_id,
        batch_id,
        lot_number,
        format,
        unit_volume_ml,
        quantity,
        stock_remaining,
        packaged_at,
        best_before_date,
    ) = r;
    PackagingRunSummary {
        run_id,
        batch_id,
        lot_number,
        format,
        unit_volume_ml,
        quantity,
        stock_remaining,
        packaged_at,
        best_before_date,
    }
}

/// Returns packaging runs for the given batches with computed stock remaining,
/// newest packaged first. Empty input yields an empty result.
pub async fn packaging_runs_by_batches(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_ids: &[Uuid],
) -> Result<Vec<PackagingRunSummary>, ApiError> {
    if batch_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "{RUN_SELECT} \
         WHERE pr.tenant_id = $1 AND pr.batch_id = ANY($2::uuid[]) \
         GROUP BY pr.id \
         ORDER BY pr.packaged_at DESC"
    );
    let rows: Vec<RunRow> = sqlx::query_as(&sql)
        .bind(tenant_id)
        .bind(batch_ids)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(run_from_row).collect())
}

/// Returns distribution movements for the given packaging runs, keyed by run id,
/// newest moved first. Empty input yields an empty map.
pub async fn movements_by_packaging_runs(
    pool: &PgPool,
    tenant_id: Uuid,
    run_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<MovementSummary>>, ApiError> {
    let mut result: HashMap<Uuid, Vec<MovementSummary>> = HashMap::new();
    if run_ids.is_empty() {
        return Ok(result);
    }
    let rows: Vec<MovementRow> = sqlx::query_as(
        "SELECT dm.id, dm.packaging_run_id, dm.movement_type, dm.quantity, \
            dm.to_location, dm.order_id, COALESCE(o.order_number, ''), dm.moved_at \
         FROM distribution_movements dm \
         LEFT JOIN orders o ON o.id = dm.order_id \
         WHERE dm.tenant_id = $1 AND dm.packaging_run_id = ANY($2::uuid[]) \
         ORDER BY dm.moved_at DESC",
    )
    .bind(tenant_id)
    .bind(run_ids)
    .fetch_all(pool)
    .await?;

    for (
        movement_id,
        run_id,
        movement_type,
        quantity,
        to_location,
        order_id,
        order_number,
        moved_at,
    ) in rows
    {
        result.entry(run_id).or_default().push(MovementSummary {
            movement_id,
            movement_type,
            quantity,
            to_location,
            order_id,
            order_number,
            moved_at,
        });
    }
    Ok(result)
}

/// Fetches a single packaging run by id with computed stock remaining,
/// tenant-scoped. Returns `not_found("packaging_run")` when absent.
pub async fn packaging_run_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    run_id: Uuid,
) -> Result<PackagingRunSummary, ApiError> {
    let sql = format!(
        "{RUN_SELECT} \
         WHERE pr.id = $1 AND pr.tenant_id = $2 \
         GROUP BY pr.id"
    );
    let row: Option<RunRow> = sqlx::query_as(&sql)
        .bind(run_id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some(r) => Ok(run_from_row(r)),
        None => Err(ApiError::not_found("packaging_run")),
    }
}

/// Fetches a single batch by id, tenant-scoped.
/// Returns `not_found("batch")` when absent.
pub async fn batch_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<BatchSummary, ApiError> {
    let row: Option<(Uuid, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, batch_number, name, status, to_char(brew_date, 'YYYY-MM-DD') \
         FROM batches \
         WHERE id = $1 AND tenant_id = $2",
    )
    .bind(batch_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some((batch_id, batch_number, name, status, brew_date)) => Ok(BatchSummary {
            batch_id,
            batch_number,
            name,
            status,
            brew_date,
        }),
        None => Err(ApiError::not_found("batch")),
    }
}

/// Returns the distinct ingredient lots consumed by a batch, ordered by name.
pub async fn ingredient_lots_by_batch(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<Vec<IngredientLotSummary>, ApiError> {
    let rows: Vec<LotRow> = sqlx::query_as(
        "SELECT DISTINCT i.id, i.lot_number, i.name, i.type, \
                COALESCE(i.supplier, ''), to_char(i.best_before_date, 'YYYY-MM-DD'), i.allergens \
             FROM batch_ingredients bi \
             JOIN ingredients i ON i.id = bi.ingredient_id \
             WHERE bi.batch_id = $1 \
               AND i.tenant_id = $2 \
             ORDER BY i.name",
    )
    .bind(batch_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(lot_id, lot_number, name, r#type, supplier, best_before_date, allergens)| {
                IngredientLotSummary {
                    lot_id,
                    lot_number,
                    name,
                    r#type,
                    supplier,
                    best_before_date,
                    allergens,
                }
            },
        )
        .collect())
}

/// Returns customers who received product from the given packaging runs (via
/// linked orders), each with their distinct order ids, ordered by customer name.
/// Empty input yields an empty result.
pub async fn affected_customers(
    pool: &PgPool,
    tenant_id: Uuid,
    run_ids: &[Uuid],
) -> Result<Vec<AffectedCustomer>, ApiError> {
    if run_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(Uuid, String, String, String, Vec<String>)> = sqlx::query_as(
        "SELECT \
            c.id, c.name, COALESCE(c.email, ''), COALESCE(c.phone, ''), \
            array_agg(DISTINCT o.id::text ORDER BY o.id::text) AS order_ids \
         FROM distribution_movements dm \
         JOIN orders o ON o.id = dm.order_id \
         JOIN customers c ON c.id = o.customer_id \
         WHERE dm.packaging_run_id = ANY($1::uuid[]) \
           AND dm.order_id IS NOT NULL \
           AND c.tenant_id = $2 \
         GROUP BY c.id, c.name, c.email, c.phone \
         ORDER BY c.name",
    )
    .bind(run_ids)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(customer_id, customer_name, email, phone, order_ids)| AffectedCustomer {
                customer_id,
                customer_name,
                email,
                phone,
                order_ids,
            },
        )
        .collect())
}
