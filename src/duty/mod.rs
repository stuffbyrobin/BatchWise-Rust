//! Beer Duty return compilation for HMRC compliance.
//!
//! Compiles `duty_events` over a calendar period into a draft duty return,
//! applies Small Producer Relief via [`crate::pkg::duty`], and supports
//! submission. Money is `i64` pence; `NUMERIC` columns are selected as `float8`
//! and `DATE` columns as `YYYY-MM-DD` strings.
//!
//! Port of the Go `internal/compliance/duty` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
