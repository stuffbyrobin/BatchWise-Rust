//! Batchwise — multi-tenant brewery management platform.
//!
//! Rust port of the Go modular monolith. The crate is organised to mirror the
//! original layout:
//!
//! - [`pkg`] — pure, dependency-free brewing physics (gravity, colour, IBU, …).
//! - [`platform`] — cross-cutting infrastructure (config, db, errors, web, …).
//! - domain modules (auth, inventory, recipe, …) are added phase by phase.

pub mod app;
pub mod auth;
pub mod inventory;
pub mod library;
pub mod pkg;
pub mod platform;
pub mod recipe;
pub mod state;
pub mod tenant;
