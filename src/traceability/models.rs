//! Traceability domain types and response shapes.
//!
//! Port of the Go `internal/traceability/traceability.go` types. `DATE` columns
//! (`best_before_date`, `brew_date`, `packaged_at`) are rendered with
//! `to_char(..., 'YYYY-MM-DD')` and carried as `String`; `moved_at` is a
//! `TIMESTAMPTZ` carried as `DateTime<Utc>`. The `allergens` column is a
//! `TEXT[]`. Money/quantities are integers.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// A condensed view of an ingredient lot for trace results.
#[derive(Debug, Clone, Serialize)]
pub struct IngredientLotSummary {
    pub lot_id: Uuid,
    pub lot_number: String,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub supplier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_before_date: Option<String>,
    pub allergens: Vec<String>,
}

/// A condensed view of a batch for trace results.
#[derive(Debug, Clone, Serialize)]
pub struct BatchSummary {
    pub batch_id: Uuid,
    pub batch_number: String,
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brew_date: Option<String>,
}

/// A condensed view of a packaging run for trace results.
#[derive(Debug, Clone, Serialize)]
pub struct PackagingRunSummary {
    pub run_id: Uuid,
    pub batch_id: Uuid,
    pub lot_number: String,
    pub format: String,
    pub unit_volume_ml: i32,
    pub quantity: i32,
    pub stock_remaining: i32,
    pub packaged_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_before_date: Option<String>,
}

/// A condensed view of a distribution movement for trace results.
#[derive(Debug, Clone, Serialize)]
pub struct MovementSummary {
    pub movement_id: Uuid,
    pub movement_type: String,
    pub quantity: i32,
    pub to_location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<Uuid>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub order_number: String,
    pub moved_at: DateTime<Utc>,
}

/// A customer who received product from an affected packaging run.
#[derive(Debug, Clone, Serialize)]
pub struct AffectedCustomer {
    pub customer_id: Uuid,
    pub customer_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub email: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub phone: String,
    pub order_ids: Vec<String>,
}

/// Full forward trace of an ingredient lot through the supply chain.
#[derive(Debug, Serialize)]
pub struct ForwardTrace {
    pub lot_number: String,
    pub ingredient: IngredientLotSummary,
    pub batches: Vec<BatchForwardNode>,
}

/// Groups a batch with its packaging runs in a forward trace.
#[derive(Debug, Serialize)]
pub struct BatchForwardNode {
    pub batch: BatchSummary,
    pub packaging_runs: Vec<PackagingForwardNode>,
}

/// Groups a packaging run with its movements in a forward trace.
#[derive(Debug, Serialize)]
pub struct PackagingForwardNode {
    pub run: PackagingRunSummary,
    pub movements: Vec<MovementSummary>,
}

/// Result of tracing a packaging run back to its ingredient lots.
#[derive(Debug, Serialize)]
pub struct BackwardTrace {
    pub run: PackagingRunSummary,
    pub batch: BatchSummary,
    pub ingredient_lots: Vec<IngredientLotSummary>,
}

/// The set of customers and orders affected by a given ingredient lot.
#[derive(Debug, Serialize)]
pub struct RecallScope {
    pub lot_number: String,
    pub ingredient: IngredientLotSummary,
    pub affected_batches: i64,
    #[serde(rename = "affected_packaging_runs")]
    pub affected_packaging: i64,
    pub affected_orders: i64,
    pub customers: Vec<AffectedCustomer>,
}
