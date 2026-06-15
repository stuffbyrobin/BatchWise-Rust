//! Sales: customers, orders, order items, and duty events. Tier-gated by the
//! `sales` feature flag.
//!
//! Port of the Go `internal/sales` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
