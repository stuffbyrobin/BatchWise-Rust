//! Traceability domain service.
//!
//! Port of the Go `internal/traceability/service.go`. The trace-graph assembly
//! is reproduced exactly: forward (lot → batches → runs → movements), backward
//! (run → batch → ingredient lots), and recall scope (lot → batches → runs →
//! affected customers/orders). The Go fire-and-forget audit write in
//! `RecallScope` is omitted (no audit module yet).

use std::collections::HashMap;

use uuid::Uuid;

use super::models::{
    BackwardTrace, BatchForwardNode, ForwardTrace, PackagingForwardNode, RecallScope,
};
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

/// Traces an ingredient lot forward through batches, packaging runs, and
/// distribution movements.
pub async fn trace_ingredient_lot(
    state: &AppState,
    tenant_id: Uuid,
    lot_number: &str,
) -> Result<ForwardTrace, ApiError> {
    let ingredient = repo::ingredient_lot_by_number(&state.pool, tenant_id, lot_number).await?;

    let batches =
        repo::batches_by_ingredient_lot(&state.pool, tenant_id, ingredient.lot_id).await?;
    let batch_ids: Vec<Uuid> = batches.iter().map(|b| b.batch_id).collect();

    let runs = repo::packaging_runs_by_batches(&state.pool, tenant_id, &batch_ids).await?;
    let run_ids: Vec<Uuid> = runs.iter().map(|r| r.run_id).collect();

    let mut mov_by_run =
        repo::movements_by_packaging_runs(&state.pool, tenant_id, &run_ids).await?;

    // Group runs by batch (preserving query order).
    let mut runs_by_batch: HashMap<Uuid, Vec<_>> = HashMap::new();
    let mut order: Vec<Uuid> = Vec::new();
    for run in runs {
        runs_by_batch.entry(run.batch_id).or_insert_with(|| {
            order.push(run.batch_id);
            Vec::new()
        });
        runs_by_batch.get_mut(&run.batch_id).unwrap().push(run);
    }

    let mut batch_nodes = Vec::with_capacity(batches.len());
    for b in batches {
        let b_runs = runs_by_batch.remove(&b.batch_id).unwrap_or_default();
        let mut pkg_nodes = Vec::with_capacity(b_runs.len());
        for run in b_runs {
            let movements = mov_by_run.remove(&run.run_id).unwrap_or_default();
            pkg_nodes.push(PackagingForwardNode { run, movements });
        }
        batch_nodes.push(BatchForwardNode {
            batch: b,
            packaging_runs: pkg_nodes,
        });
    }

    Ok(ForwardTrace {
        lot_number: lot_number.to_string(),
        ingredient,
        batches: batch_nodes,
    })
}

/// Traces a packaging run backward to the batch and ingredient lots it came from.
pub async fn trace_packaging_run(
    state: &AppState,
    tenant_id: Uuid,
    run_id: Uuid,
) -> Result<BackwardTrace, ApiError> {
    let run = repo::packaging_run_by_id(&state.pool, tenant_id, run_id).await?;
    let batch = repo::batch_by_id(&state.pool, tenant_id, run.batch_id).await?;
    let ingredient_lots =
        repo::ingredient_lots_by_batch(&state.pool, tenant_id, run.batch_id).await?;

    Ok(BackwardTrace {
        run,
        batch,
        ingredient_lots,
    })
}

/// Computes the recall scope (affected batches, packaging runs, orders, and
/// customers) for a given ingredient lot.
pub async fn recall_scope(
    state: &AppState,
    tenant_id: Uuid,
    lot_number: &str,
) -> Result<RecallScope, ApiError> {
    let ingredient = repo::ingredient_lot_by_number(&state.pool, tenant_id, lot_number).await?;

    let batches =
        repo::batches_by_ingredient_lot(&state.pool, tenant_id, ingredient.lot_id).await?;
    let batch_ids: Vec<Uuid> = batches.iter().map(|b| b.batch_id).collect();

    let runs = repo::packaging_runs_by_batches(&state.pool, tenant_id, &batch_ids).await?;
    let run_ids: Vec<Uuid> = runs.iter().map(|r| r.run_id).collect();

    let customers = repo::affected_customers(&state.pool, tenant_id, &run_ids).await?;

    let affected_orders: i64 = customers.iter().map(|c| c.order_ids.len() as i64).sum();

    // The Go RecallScope.Write audit call is omitted (no audit module yet).

    Ok(RecallScope {
        lot_number: lot_number.to_string(),
        ingredient,
        affected_batches: batches.len() as i64,
        affected_packaging: runs.len() as i64,
        affected_orders,
        customers,
    })
}
