//! Pure brewing-physics calculations.
//!
//! Every module here is deterministic, side-effect free, and has zero
//! dependencies on the rest of the crate — they are the Rust equivalents of the
//! Go `pkg/` packages and may be used by any service layer.

pub mod allergen;
pub mod bitterness;
pub mod color;
pub mod duty;
pub mod energy;
pub mod gravity;
pub mod nutrition;
pub mod water;
