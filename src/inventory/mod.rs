//! Ingredient lots and FIFO-by-best-before-date stock deduction.
//!
//! Port of the Go `internal/inventory` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
