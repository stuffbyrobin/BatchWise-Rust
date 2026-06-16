//! Live recipe calculated values (OG/FG/ABV/IBU/colour) for the editor.
//!
//! Mirrors the backend's `src/recipe/calc.rs` orchestration so the frontend can
//! preview the same numbers the server will compute on save. The underlying
//! math is the shared `pkg` (gravity/color/bitterness); only this ~40-line
//! orchestration is duplicated here, because `recipe::calc` depends on
//! `recipe::models` (which derive sqlx and can't target wasm). **Keep in sync
//! with `src/recipe/calc.rs`.**

use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::pkg::{bitterness, color, gravity};

#[derive(Deserialize)]
struct FermentableInput {
    amount: f64,
    unit: String,
    #[serde(default)]
    potential_ppg: Option<f64>,
    #[serde(default)]
    color_ebc: Option<f64>,
}

#[derive(Deserialize)]
struct HopInput {
    amount: f64,
    unit: String,
    #[serde(default)]
    alpha_acid_pct: f64,
    #[serde(default)]
    boil_time_minutes: f64,
    #[serde(default)]
    form: Option<String>,
    #[serde(default)]
    r#use: Option<String>,
}

#[derive(Deserialize)]
struct RecipeInput {
    batch_size_liters: f64,
    #[serde(default)]
    efficiency_pct: Option<f64>,
    /// First yeast's attenuation (%), if any.
    #[serde(default)]
    attenuation_pct: Option<f64>,
    #[serde(default)]
    fermentables: Vec<FermentableInput>,
    #[serde(default)]
    hops: Vec<HopInput>,
}

/// The computed recipe values, surfaced to JS with camelCase getters.
#[wasm_bindgen]
pub struct RecipeCalcs {
    calc_og: f64,
    calc_fg: f64,
    calc_abv_pct: f64,
    calc_ibu: f64,
    calc_color_ebc: f64,
}

#[wasm_bindgen]
impl RecipeCalcs {
    #[wasm_bindgen(getter)]
    pub fn calc_og(&self) -> f64 {
        self.calc_og
    }
    #[wasm_bindgen(getter)]
    pub fn calc_fg(&self) -> f64 {
        self.calc_fg
    }
    #[wasm_bindgen(getter)]
    pub fn calc_abv_pct(&self) -> f64 {
        self.calc_abv_pct
    }
    #[wasm_bindgen(getter)]
    pub fn calc_ibu(&self) -> f64 {
        self.calc_ibu
    }
    #[wasm_bindgen(getter)]
    pub fn calc_color_ebc(&self) -> f64 {
        self.calc_color_ebc
    }
}

/// Computes OG/FG/ABV/IBU/colour from a recipe-form JSON payload.
#[wasm_bindgen(js_name = computeRecipeCalcs)]
pub fn compute_recipe_calcs(input_json: &str) -> Result<RecipeCalcs, JsError> {
    let r: RecipeInput =
        serde_json::from_str(input_json).map_err(|e| JsError::new(&e.to_string()))?;

    let calc_og = compute_og(&r);
    let calc_color_ebc = compute_color_ebc(&r.fermentables, r.batch_size_liters);

    // FG from the first yeast's attenuation, else 75%.
    let attenuation = r.attenuation_pct.unwrap_or(75.0);
    let calc_fg = calc_og - (calc_og - 1.0) * (attenuation / 100.0);

    let calc_abv_pct = if calc_og > 1.0 {
        gravity::calculate_abv(calc_og, calc_fg).unwrap_or(0.0)
    } else {
        0.0
    };

    let calc_ibu = compute_ibu(&r.hops, r.batch_size_liters, calc_og);

    Ok(RecipeCalcs {
        calc_og,
        calc_fg,
        calc_abv_pct,
        calc_ibu,
        calc_color_ebc,
    })
}

fn compute_og(r: &RecipeInput) -> f64 {
    if r.fermentables.is_empty() {
        return 1.0;
    }
    let efficiency = r.efficiency_pct.unwrap_or(75.0);
    let calc = gravity::Calculator::new(gravity::BrewingStandard::Iob);

    let mut total_points = 0.0;
    let mut any = false;
    for f in &r.fermentables {
        let ppg = match f.potential_ppg {
            Some(p) if p > 0.0 => p,
            _ => continue,
        };
        let amt_kg = if f.unit == "g" {
            f.amount / 1000.0
        } else {
            f.amount
        };
        if let Ok(res) = calc.calculate(gravity::MaltAddition {
            mass_amount: amt_kg,
            mass_unit: gravity::MassUnit::Kg,
            potential_value: ppg,
            potential_unit: gravity::PotentialUnit::Ppg,
            volume_amount: r.batch_size_liters,
            volume_unit: gravity::VolumeUnit::Litres,
            efficiency,
        }) {
            total_points += res.metric_points;
            any = true;
        }
    }
    if !any {
        return 1.0;
    }
    1.0 + total_points / 1000.0
}

fn compute_color_ebc(ferms: &[FermentableInput], batch_vol_l: f64) -> f64 {
    let entries: Vec<color::FermentableEntry> = ferms
        .iter()
        .filter_map(|f| {
            f.color_ebc.map(|ebc| color::FermentableEntry {
                amount_kg: if f.unit == "g" {
                    f.amount / 1000.0
                } else {
                    f.amount
                },
                color_ebc: ebc,
            })
        })
        .collect();
    color::srm_to_ebc(color::calculate_srm(&entries, batch_vol_l))
}

fn compute_ibu(hops: &[HopInput], batch_vol_l: f64, og: f64) -> f64 {
    let additions: Vec<bitterness::HopAddition> = hops
        .iter()
        .map(|h| bitterness::HopAddition {
            amount_g: if h.unit == "kg" {
                h.amount * 1000.0
            } else {
                h.amount
            },
            alpha_acid_pct: h.alpha_acid_pct,
            boil_time_minutes: h.boil_time_minutes,
            form: h.form.clone().unwrap_or_default(),
            use_: h.r#use.clone().unwrap_or_default(),
        })
        .collect();
    bitterness::calculate_tinseth(&additions, batch_vol_l, og)
}
