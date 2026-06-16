//! Equipment register, maintenance schedules, and maintenance events.
//! Tier-gated by the `equipment_maintenance` feature flag.
//!
//! Port of the Go `internal/equipment` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
