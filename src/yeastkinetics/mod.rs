//! Yeast fermentation kinetics profiles (CRUD + closest-temperature lookup).
//!
//! Reads of a kinetics row by id are tenant-scoped; writes are tenant-scoped.
//! [`service::find_closest_for_yeast`] additionally falls back to the system
//! tenant when the caller's tenant has no rows for the yeast.
//!
//! Port of the Go `internal/yeastkinetics` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
