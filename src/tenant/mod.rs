//! Tenant management: reading and updating tenant data and feature flags.
//!
//! Port of the Go `internal/tenant` package.

pub mod handler;
pub mod models;
pub mod presets;
pub mod repository;
pub mod service;

pub use handler::routes;
