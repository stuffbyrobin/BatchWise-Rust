//! Shared reference data: beer styles, equipment profiles, mash profiles,
//! yeasts, and library fermentables.
//!
//! Port of the Go `internal/library` package. Reads return the union of the
//! system-tenant rows and the caller's tenant rows; writes target only the
//! caller's tenant (cross-tenant modifications resolve to `not_found`).

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
