//! Re-exports the backend's pure physics modules verbatim via `#[path]`, so the
//! WASM bundle and the server compile the same source. Paths are relative to this
//! file's directory (`wasm/src/pkg/`).

#[path = "../../../src/pkg/bitterness.rs"]
pub mod bitterness;
#[path = "../../../src/pkg/color.rs"]
pub mod color;
#[path = "../../../src/pkg/duty.rs"]
pub mod duty;
#[path = "../../../src/pkg/gravity.rs"]
pub mod gravity;
#[path = "../../../src/pkg/nutrition.rs"]
pub mod nutrition;
#[path = "../../../src/pkg/water.rs"]
pub mod water;
