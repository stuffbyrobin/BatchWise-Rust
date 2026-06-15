//! Water chemistry: named mineral profiles, saved adjustment sessions, and a
//! stateless treatment calculator. Auth-only (no feature gate).
//!
//! Port of the Go `internal/water` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
