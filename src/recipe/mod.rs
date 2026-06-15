//! Recipe management with nested fermentables, hops, yeasts, and mash steps.
//! Calculated values (OG, FG, ABV, IBU, EBC) are recomputed on every write.
//!
//! Port of the Go `internal/recipe` package.

pub mod calc;
pub mod handler;
pub mod import_beerxml;
pub mod import_brewfather;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
