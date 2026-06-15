//! Procurement: suppliers and purchase orders (with lines and receipts).
//! Tier-gated by the `procurement` feature flag.
//!
//! Port of the Go `internal/procurement` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
