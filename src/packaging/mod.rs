//! Packaging runs and distribution movements: records packaging of a batch into
//! a container format and the subsequent stock movements. Tier-gated by the
//! `packaging` feature flag.
//!
//! Port of the Go `internal/packaging` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
