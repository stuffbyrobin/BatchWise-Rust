//! UK label-compliance records for batches.
//!
//! On creation a record is auto-populated from the batch's recipe snapshot, the
//! tenant's identity/address, and the computed allergen declaration (via
//! [`crate::allergens`]); voluntary nutrition fields are derived from ABV via
//! [`crate::pkg::nutrition`]. Records can be edited while `draft` and become
//! immutable once `approved`. Money is unused here; `NUMERIC` columns are
//! selected as `float8` and the `DATE` column as a `YYYY-MM-DD` string; the
//! `allergens` column is a Postgres `TEXT[]`.
//!
//! Port of the Go `internal/compliance/labels` package.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
