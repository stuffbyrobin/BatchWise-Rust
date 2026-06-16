//! Live water-treatment / mash-pH preview for the recipe editor.
//!
//! Mirrors the backend's `src/water/service::calculate` pure logic so the
//! frontend can preview the same treated-water profile and predicted mash pH
//! the server will compute on save. The underlying chemistry is the shared
//! `pkg::water` (included verbatim via `#[path]`); only this mapping
//! orchestration is duplicated here, because `water::service` depends on
//! `water::models` + `AppState` (sqlx/HTTP) which can't target wasm. **Keep the
//! mineral/grain mapping in sync with `src/water/service.rs`.**

use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::pkg::water as pw;

#[derive(Deserialize)]
struct SourceInput {
    #[serde(default)]
    calcium_ppm: f64,
    #[serde(default)]
    magnesium_ppm: f64,
    #[serde(default)]
    sodium_ppm: f64,
    #[serde(default)]
    sulfate_ppm: f64,
    #[serde(default)]
    chloride_ppm: f64,
    #[serde(default)]
    bicarbonate_ppm: f64,
}

#[derive(Deserialize)]
struct MineralInput {
    r#type: String,
    #[serde(default)]
    amount: f64,
}

#[derive(Deserialize)]
struct GrainInput {
    grain_type: String,
    #[serde(default)]
    weight_kg: f64,
    #[serde(default)]
    colour_lovibond: f64,
}

#[derive(Deserialize)]
struct WaterInput {
    source: SourceInput,
    volume_liters: f64,
    #[serde(default)]
    minerals: Vec<MineralInput>,
    #[serde(default)]
    grains: Vec<GrainInput>,
}

/// The computed water-treatment values, surfaced to JS with snake_case getters.
#[wasm_bindgen]
pub struct WaterTreatment {
    calcium_ppm: f64,
    magnesium_ppm: f64,
    sodium_ppm: f64,
    sulfate_ppm: f64,
    chloride_ppm: f64,
    bicarbonate_ppm: f64,
    alkalinity: f64,
    residual_alk: f64,
    sulfate_to_chloride: f64,
    mash_ph: f64,
}

#[wasm_bindgen]
impl WaterTreatment {
    #[wasm_bindgen(getter)]
    pub fn calcium_ppm(&self) -> f64 {
        self.calcium_ppm
    }
    #[wasm_bindgen(getter)]
    pub fn magnesium_ppm(&self) -> f64 {
        self.magnesium_ppm
    }
    #[wasm_bindgen(getter)]
    pub fn sodium_ppm(&self) -> f64 {
        self.sodium_ppm
    }
    #[wasm_bindgen(getter)]
    pub fn sulfate_ppm(&self) -> f64 {
        self.sulfate_ppm
    }
    #[wasm_bindgen(getter)]
    pub fn chloride_ppm(&self) -> f64 {
        self.chloride_ppm
    }
    #[wasm_bindgen(getter)]
    pub fn bicarbonate_ppm(&self) -> f64 {
        self.bicarbonate_ppm
    }
    #[wasm_bindgen(getter)]
    pub fn alkalinity(&self) -> f64 {
        self.alkalinity
    }
    #[wasm_bindgen(getter)]
    pub fn residual_alk(&self) -> f64 {
        self.residual_alk
    }
    #[wasm_bindgen(getter)]
    pub fn sulfate_to_chloride(&self) -> f64 {
        self.sulfate_to_chloride
    }
    #[wasm_bindgen(getter)]
    pub fn mash_ph(&self) -> f64 {
        self.mash_ph
    }
}

/// Maps a mineral salt string to its [`pw::MineralType`]. Unknown salts are
/// dropped (this is a live preview, not a strict server-side validation).
/// Mirrors `src/water/service::map_minerals`, plus accepts the frontend's
/// `Ca(OH)2` spelling for slaked lime.
fn map_mineral_type(t: &str) -> Option<pw::MineralType> {
    Some(match t {
        "CaSO4" => pw::MineralType::Gypsum,
        "CaCl2" => pw::MineralType::CalciumCl,
        "CaCO3" => pw::MineralType::Chalk,
        "MgSO4" => pw::MineralType::Epsom,
        "MgCl2" => pw::MineralType::MagnesiumCl,
        "NaHCO3" => pw::MineralType::BakingSoda,
        "NaCl" => pw::MineralType::TableSalt,
        "Na2SO4" => pw::MineralType::SodiumSulfate,
        "CaOH2" | "Ca(OH)2" => pw::MineralType::SlakedLime,
        _ => return None,
    })
}

/// Maps a grain-type string to its [`pw::GrainType`]. Unknown types are dropped.
/// Mirrors `src/water/service::map_grains`.
fn map_grain_type(t: &str) -> Option<pw::GrainType> {
    Some(match t {
        "base" => pw::GrainType::Base,
        "crystal" => pw::GrainType::Crystal,
        "roast" => pw::GrainType::Roast,
        "acid" => pw::GrainType::Acid,
        _ => return None,
    })
}

/// Computes the treated-water profile + predicted mash pH from a JSON payload.
#[wasm_bindgen(js_name = computeWaterTreatment)]
pub fn compute_water_treatment(input_json: &str) -> Result<WaterTreatment, JsError> {
    let input: WaterInput =
        serde_json::from_str(input_json).map_err(|e| JsError::new(&e.to_string()))?;

    let source = pw::WaterProfile {
        calcium: input.source.calcium_ppm,
        magnesium: input.source.magnesium_ppm,
        sodium: input.source.sodium_ppm,
        sulfate: input.source.sulfate_ppm,
        chloride: input.source.chloride_ppm,
        bicarbonate: input.source.bicarbonate_ppm,
    };

    let minerals: Vec<pw::MineralAddition> = input
        .minerals
        .iter()
        .filter_map(|m| {
            map_mineral_type(&m.r#type).map(|mineral_type| pw::MineralAddition {
                mineral_type,
                amount: m.amount,
            })
        })
        .collect();

    let grains: Vec<pw::GrainAddition> = input
        .grains
        .iter()
        .filter_map(|g| {
            map_grain_type(&g.grain_type).map(|grain_type| pw::GrainAddition {
                name: String::new(),
                grain_type,
                weight: g.weight_kg,
                colour: g.colour_lovibond,
            })
        })
        .collect();

    let mash = if grains.is_empty() {
        None
    } else {
        Some(pw::MashParameters {
            water_volume: input.volume_liters,
            grains,
            acids: vec![],
        })
    };

    let calc = pw::calculate_water_treatment(source, input.volume_liters, &minerals, mash.as_ref())
        .map_err(|e| JsError::new(&e.to_string()))?;

    let mash_ph = if mash.is_some() { calc.mash_ph } else { 0.0 };

    Ok(WaterTreatment {
        calcium_ppm: calc.final_profile.calcium,
        magnesium_ppm: calc.final_profile.magnesium,
        sodium_ppm: calc.final_profile.sodium,
        sulfate_ppm: calc.final_profile.sulfate,
        chloride_ppm: calc.final_profile.chloride,
        bicarbonate_ppm: calc.final_profile.bicarbonate,
        alkalinity: calc.alkalinity,
        residual_alk: calc.residual_alk,
        sulfate_to_chloride: calc.sulfate_to_chloride,
        mash_ph,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden fixture guarding drift in the mineral/grain mapping + the shared
    // `pkg::water` math. Values were computed once from FIXTURE and baked in; if
    // this module's mapping or the underlying chemistry changes, this test fails.
    const GOLDEN_CALCIUM: f64 = 50.085_181_752_016_965;
    const GOLDEN_MAGNESIUM: f64 = 5.0;
    const GOLDEN_SODIUM: f64 = 10.0;
    const GOLDEN_SULFATE: f64 = 50.117_599_529_895_7;
    const GOLDEN_CHLORIDE: f64 = 50.063_885_384_754_01;
    const GOLDEN_BICARBONATE: f64 = 100.0;
    const GOLDEN_ALKALINITY: f64 = 81.967_213_114_754_1;
    const GOLDEN_RESIDUAL_ALK: f64 = 43.251_867_789_288_38;
    const GOLDEN_SC: f64 = 1.001_072_912_034_870_7;
    const GOLDEN_MASH_PH: f64 = 6.024_410_191_256_83;

    const FIXTURE: &str = r#"{
        "source": {
            "calcium_ppm": 50, "magnesium_ppm": 5, "sodium_ppm": 10,
            "sulfate_ppm": 50, "chloride_ppm": 50, "bicarbonate_ppm": 100
        },
        "volume_liters": 30,
        "minerals": [
            {"type": "CaSO4", "amount": 5},
            {"type": "CaCl2", "amount": 3}
        ],
        "grains": [
            {"grain_type": "base", "weight_kg": 5.0, "colour_lovibond": 4.0},
            {"grain_type": "crystal", "weight_kg": 0.5, "colour_lovibond": 60.0}
        ]
    }"#;

    #[test]
    fn matches_golden_fixture() {
        let r = compute_water_treatment(FIXTURE).expect("compute");
        assert!(
            (r.calcium_ppm() - GOLDEN_CALCIUM).abs() < 1e-9,
            "calcium {}",
            r.calcium_ppm()
        );
        assert!(
            (r.magnesium_ppm() - GOLDEN_MAGNESIUM).abs() < 1e-9,
            "magnesium {}",
            r.magnesium_ppm()
        );
        assert!(
            (r.sodium_ppm() - GOLDEN_SODIUM).abs() < 1e-9,
            "sodium {}",
            r.sodium_ppm()
        );
        assert!(
            (r.sulfate_ppm() - GOLDEN_SULFATE).abs() < 1e-9,
            "sulfate {}",
            r.sulfate_ppm()
        );
        assert!(
            (r.chloride_ppm() - GOLDEN_CHLORIDE).abs() < 1e-9,
            "chloride {}",
            r.chloride_ppm()
        );
        assert!(
            (r.bicarbonate_ppm() - GOLDEN_BICARBONATE).abs() < 1e-9,
            "bicarbonate {}",
            r.bicarbonate_ppm()
        );
        assert!(
            (r.alkalinity() - GOLDEN_ALKALINITY).abs() < 1e-9,
            "alkalinity {}",
            r.alkalinity()
        );
        assert!(
            (r.residual_alk() - GOLDEN_RESIDUAL_ALK).abs() < 1e-9,
            "residual_alk {}",
            r.residual_alk()
        );
        assert!(
            (r.sulfate_to_chloride() - GOLDEN_SC).abs() < 1e-9,
            "sulfate_to_chloride {}",
            r.sulfate_to_chloride()
        );
        assert!(
            (r.mash_ph() - GOLDEN_MASH_PH).abs() < 1e-9,
            "mash_ph {}",
            r.mash_ph()
        );
    }
}
