//! Fermentation vessels (fermenters): CRUD. Batches are assigned to a fermenter
//! for the planning/Gantt view via `batches.fermenter_id`.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
