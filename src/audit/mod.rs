//! Compliance audit log (`/compliance-audit`).
//!
//! Port of the Go `internal/compliance/audit` package. A cross-cutting,
//! append-only log written fire-and-forget by other modules (packaging,
//! traceability, labels, duty, allergens) via [`service::write`] and read back
//! through the read-only HTTP endpoints. Not feature-gated.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
