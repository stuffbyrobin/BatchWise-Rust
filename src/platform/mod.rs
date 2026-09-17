//! Cross-cutting infrastructure shared by every domain module.
//!
//! Port of the Go `internal/platform` packages, adapted to axum + sqlx:
//!
//! - [`config`] — environment-variable configuration loading and validation.
//! - [`context`] — request-scoped tenant / user / request-id values.
//! - [`database`] — the `sqlx` Postgres pool and migration runner.
//! - [`errors`] — the [`errors::ApiError`] type and its JSON rendering.
//! - [`features`] — per-tenant feature-flag cache for the tier gate.
//! - [`logger`] — `tracing` subscriber setup.
//! - [`pagination`] — page/page-size clamping, saturating offsets, and the
//!   shared [`pagination::Page`] envelope.
//! - [`refs`] — tenant checks for ids supplied in request bodies.
//! - [`sort`] — allow-listed `?sort=` parsing into `ORDER BY` fragments.
//! - [`sql`] — small SQL helpers (LIKE pattern escaping).
//! - [`web`] — JSON request/response helpers.

pub mod authz;
pub mod config;
pub mod context;
pub mod database;
pub mod errors;
pub mod features;
pub mod logger;
pub mod middleware;
pub mod pagination;
pub mod redis_limits;
pub mod refs;
pub mod roles;
pub mod seed;
pub mod sort;
pub mod sql;
pub mod web;
