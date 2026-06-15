//! Cost rate management, batch cost computation, and cost report generation.
//!
//! Money is `i64` pence; `batch_costs.total_cost_pence` is a DB-generated column
//! (read, never written). The duty estimate is delegated to [`crate::pkg::duty`].
//!
//! Port of the Go `internal/reporting` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
