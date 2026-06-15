//! Cross-cutting infrastructure shared by every domain module.
//!
//! Port of the Go `internal/platform` packages, adapted to axum + sqlx:
//!
//! - [`config`] — environment-variable configuration loading and validation.
//! - [`context`] — request-scoped tenant / user / request-id values.
//! - [`database`] — the `sqlx` Postgres pool and migration runner.
//! - [`errors`] — the [`errors::ApiError`] type and its JSON rendering.
//! - [`logger`] — `tracing` subscriber setup.
//! - [`web`] — JSON request/response helpers.

pub mod config;
pub mod context;
pub mod database;
pub mod errors;
pub mod logger;
pub mod middleware;
pub mod seed;
pub mod web;
