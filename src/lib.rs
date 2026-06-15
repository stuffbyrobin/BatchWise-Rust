//! Batchwise — multi-tenant brewery management platform.
//!
//! Rust port of the Go modular monolith. The crate is organised to mirror the
//! original layout:
//!
//! - [`pkg`] — pure, dependency-free brewing physics (gravity, colour, IBU, …).
//! - [`platform`] — cross-cutting infrastructure (config, db, errors, web, …).
//! - domain modules (auth, inventory, recipe, …) are added phase by phase.

pub mod allergens;
pub mod app;
pub mod audit;
pub mod auth;
pub mod batch;
pub mod calendar;
pub mod dashboard;
pub mod duty;
pub mod equipment;
pub mod fermentation;
pub mod inventory;
pub mod labeldesign;
pub mod labels;
pub mod library;
pub mod openapi;
pub mod packaging;
pub mod pkg;
pub mod platform;
pub mod procurement;
pub mod recipe;
pub mod reporting;
pub mod sales;
pub mod state;
pub mod tenant;
pub mod traceability;
pub mod tracking;
pub mod water;
pub mod yeastbanking;
pub mod yeastkinetics;
