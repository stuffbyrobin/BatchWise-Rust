//! Yeast banking: yeast bank entries and propagation events.
//! Tier-gated by the `yeast_banking` feature flag.
//!
//! Port of the Go `internal/yeastbanking` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
