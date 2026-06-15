//! Recipe calculated values (OG/FG/ABV/IBU/colour) via the `pkg` physics.
//!
//! Port of the calculation helpers in the Go `internal/recipe/service.go`.

use super::models::{CalculatedValues, Fermentable, Hop, Recipe, Yeast};
use crate::pkg::{bitterness, color, gravity};

/// Runs the full physics pipeline for a recipe and its grain/hop/yeast bills.
pub fn compute_calcs(
    rec: &Recipe,
    ferms: &[Fermentable],
    hops: &[Hop],
    yeasts: &[Yeast],
) -> CalculatedValues {
    let calc_og = compute_og(rec, ferms);
    let calc_color_ebc = compute_color_ebc(ferms, rec.batch_size_liters);

    // FG from the first yeast's attenuation, else 75%.
    let attenuation = yeasts
        .first()
        .and_then(|y| y.attenuation_pct)
        .unwrap_or(75.0);
    let calc_fg = calc_og - (calc_og - 1.0) * (attenuation / 100.0);

    let calc_abv_pct = if calc_og > 1.0 {
        gravity::calculate_abv(calc_og, calc_fg).unwrap_or(0.0)
    } else {
        0.0
    };

    let calc_ibu = compute_ibu(hops, rec.batch_size_liters, calc_og);

    CalculatedValues {
        calc_og,
        calc_fg,
        calc_abv_pct,
        calc_ibu,
        calc_color_ebc,
    }
}

/// Calculates wort OG from the grain bill (sum of gravity points).
fn compute_og(rec: &Recipe, ferms: &[Fermentable]) -> f64 {
    if ferms.is_empty() {
        return 1.0;
    }
    let efficiency = rec.efficiency_pct.unwrap_or(75.0);
    let calc = gravity::Calculator::new(gravity::BrewingStandard::Iob);

    let mut total_points = 0.0;
    let mut any = false;
    for f in ferms {
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
            volume_amount: rec.batch_size_liters,
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

/// Calculates beer colour in EBC from the grain bill.
fn compute_color_ebc(ferms: &[Fermentable], batch_vol_l: f64) -> f64 {
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

/// Calculates IBU via the Tinseth formula.
fn compute_ibu(hops: &[Hop], batch_vol_l: f64, og: f64) -> f64 {
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
