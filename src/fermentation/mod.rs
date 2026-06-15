//! Fermentation readings logged against batches (`/batches/{id}/fermentation`).
//!
//! Port of the Go `internal/fermentation` package. Gated by the `fermentation`
//! feature flag; the router is merged into the `/batches` nest.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
