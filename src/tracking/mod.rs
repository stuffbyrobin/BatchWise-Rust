//! Container asset tracking (keg/cask management): assets, event logs, and QR
//! codes. Tier-gated by the `tracking` feature flag.
//!
//! Port of the Go `internal/tracking` package.

pub mod handler;
pub mod models;
pub mod qr;
pub mod repository;
pub mod service;

pub use handler::routes;
