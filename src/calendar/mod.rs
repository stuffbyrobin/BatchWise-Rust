//! Brewing calendar events: CRUD plus batch-generated event creation.
//!
//! Port of the Go `internal/calendar` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
