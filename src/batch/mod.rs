//! Brewing batches: recipe snapshots, status FSM, calendar-event generation,
//! and deferred inventory deduction.
//!
//! Port of the Go `internal/batch` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
