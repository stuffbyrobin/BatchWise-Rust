//! User authentication: registration, login, JWT issuance, refresh-token
//! rotation, and profile management.
//!
//! Port of the Go `internal/auth` package.

pub mod cleanup;
pub mod handler;
pub mod jwt;
pub mod models;
pub mod password;
pub mod refresh;
pub mod repository;
pub mod service;

pub use handler::routes;
