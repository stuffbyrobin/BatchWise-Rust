//! Label design: brand assets (logos), brand profiles, and label/clip/lens
//! design instances rendered to print-ready PDF. Tier-gated by the
//! `label_design` feature flag.
//!
//! Port of the Go `internal/labeldesign` package. The PDF/render core lives in
//! [`crate::pkg::labelkit`]; this module owns persistence, validation, and the
//! resolution of brand + compliance/recipe fields into a render model.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
