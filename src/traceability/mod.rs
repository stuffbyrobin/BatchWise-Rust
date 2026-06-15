//! Forward/backward supply-chain tracing and recall scope.
//!
//! Read-only trace queries (ingredient lot → batches → packaging runs →
//! distribution movements, and the reverse), plus recall scope (affected
//! customers/orders for a lot). All routes are gated by the `traceability`
//! feature flag and require auth. The Go fire-and-forget audit write in
//! `RecallScope` is omitted (no audit module yet).
//!
//! Port of the Go `internal/traceability` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
