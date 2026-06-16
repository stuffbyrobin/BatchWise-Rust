//! WebAssembly bindings for the BatchWise brewing-physics calculations.
//!
//! The `pkg` modules below are the **same source files** the backend compiles
//! (`../../src/pkg/*.rs`), included via `#[path]` so there is a single source of
//! truth. Each is pure `std` Rust with no external deps, so it cross-compiles to
//! `wasm32-unknown-unknown` cleanly. Only a curated, scalar-friendly subset is
//! exposed across the JS boundary for this prototype.

#![allow(clippy::empty_docs)]
// The `pkg` modules are included whole; only a curated subset is exposed, so
// unexposed items (and their helpers) are legitimately unused here.
#![allow(dead_code)]

use wasm_bindgen::prelude::*;

/// The shared physics modules, compiled verbatim from the backend's `src/pkg`.
/// Declared in `src/pkg/mod.rs` so the `#[path]` includes resolve through an
/// existing directory.
mod pkg;

/// Live recipe OG/FG/ABV/IBU/colour (mirrors `src/recipe/calc.rs`).
mod recipe_calc;
pub use recipe_calc::{compute_recipe_calcs, RecipeCalcs};

/// Live water-treatment / mash-pH preview (mirrors `src/water/service.rs`).
mod water_calc;
pub use water_calc::{compute_water_treatment, WaterTreatment};

/// Maps a physics error (anything `Display`) into a thrown JS `Error`.
fn js_err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

// ——— Gravity ————————————————————————————————————————————————————————————————

/// ABV % from original and final gravity (e.g. `1.050, 1.010` → `5.25`).
#[wasm_bindgen(js_name = calculateAbv)]
pub fn calculate_abv(og: f64, fg: f64) -> Result<f64, JsError> {
    pkg::gravity::calculate_abv(og, fg).map_err(js_err)
}

/// Apparent attenuation % from original and final gravity.
#[wasm_bindgen(js_name = calculateAttenuation)]
pub fn calculate_attenuation(og: f64, fg: f64) -> Result<f64, JsError> {
    pkg::gravity::calculate_attenuation(og, fg).map_err(js_err)
}

/// Estimated calories per 12 oz from original and final gravity.
#[wasm_bindgen(js_name = calculateCalories)]
pub fn calculate_calories(og: f64, fg: f64) -> Result<f64, JsError> {
    pkg::gravity::calculate_calories(og, fg).map_err(js_err)
}

/// Specific gravity → degrees Plato.
#[wasm_bindgen(js_name = sgToPlato)]
pub fn sg_to_plato(sg: f64) -> f64 {
    pkg::gravity::sg_to_plato(sg)
}

/// Degrees Plato → specific gravity.
#[wasm_bindgen(js_name = platoToSg)]
pub fn plato_to_sg(plato: f64) -> f64 {
    pkg::gravity::plato_to_sg(plato)
}

// ——— Colour —————————————————————————————————————————————————————————————————

/// SRM → EBC.
#[wasm_bindgen(js_name = srmToEbc)]
pub fn srm_to_ebc(srm: f64) -> f64 {
    pkg::color::srm_to_ebc(srm)
}

/// EBC → SRM.
#[wasm_bindgen(js_name = ebcToSrm)]
pub fn ebc_to_srm(ebc: f64) -> f64 {
    pkg::color::ebc_to_srm(ebc)
}

/// Degrees Lovibond → EBC.
#[wasm_bindgen(js_name = lovibondToEbc)]
pub fn lovibond_to_ebc(lovibond: f64) -> f64 {
    pkg::color::lovibond_to_ebc(lovibond)
}

// ——— Nutrition ——————————————————————————————————————————————————————————————

/// Energy (kJ) per 100 ml from ABV %.
#[wasm_bindgen(js_name = energyKjPer100ml)]
pub fn energy_kj_per_100ml(abv_pct: f64) -> f64 {
    pkg::nutrition::energy_kj_per_100ml(abv_pct)
}

/// Energy (kcal) per 100 ml from ABV %.
#[wasm_bindgen(js_name = energyKcalPer100ml)]
pub fn energy_kcal_per_100ml(abv_pct: f64) -> f64 {
    pkg::nutrition::energy_kcal_per_100ml(abv_pct)
}

/// UK alcohol units for a serving (ABV % and volume in ml).
#[wasm_bindgen(js_name = alcoholUnits)]
pub fn alcohol_units(abv_pct: f64, volume_ml: f64) -> f64 {
    pkg::nutrition::alcohol_units(abv_pct, volume_ml)
}

// ——— Duty ———————————————————————————————————————————————————————————————————

/// UK beer duty in **pence** for a volume (litres) at a given ABV %.
#[wasm_bindgen(js_name = calculateBeerDutyGbPence)]
pub fn calculate_beer_duty_gb_pence(volume_liters: f64, abv_pct: f64) -> f64 {
    pkg::duty::calculate_beer_duty_gb(volume_liters, abv_pct) as f64
}

/// Small Producer Relief rate (0.0–1.0) for an annual production in hl/year.
#[wasm_bindgen(js_name = sprReliefRate)]
pub fn spr_relief_rate(annual_production_hl_pa: f64) -> f64 {
    pkg::duty::spr_relief_rate(annual_production_hl_pa)
}
