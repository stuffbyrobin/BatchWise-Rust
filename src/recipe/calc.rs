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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    // Shared golden fixture — kept identical to the parity test in
    // `wasm/src/recipe_calc.rs::tests`. If this module's math changes, update
    // both. Drift between the backend and the WASM editor preview fails one side.
    pub(crate) const GOLDEN_OG: f64 = 1.055_129_738_824;
    pub(crate) const GOLDEN_FG: f64 = 1.012_128_542_541_28;
    pub(crate) const GOLDEN_ABV: f64 = 5.643_907_012_106_99;
    pub(crate) const GOLDEN_IBU: f64 = 39.072_379_101_953_366;
    pub(crate) const GOLDEN_EBC: f64 = 17.699_029_979_284_365;

    fn ferm(step_order: i32, amount: f64, ppg: f64, ebc: f64) -> Fermentable {
        Fermentable {
            id: Uuid::nil(),
            recipe_id: Uuid::nil(),
            step_order,
            name: "F".into(),
            amount,
            unit: "kg".into(),
            color_ebc: Some(ebc),
            potential_ppg: Some(ppg),
            r#type: None,
            addition: None,
            inventory_lot_id: None,
        }
    }

    fn hop(step_order: i32, amount_g: f64, aa: f64, minutes: f64) -> Hop {
        Hop {
            id: Uuid::nil(),
            recipe_id: Uuid::nil(),
            step_order,
            name: "H".into(),
            amount: amount_g,
            unit: "g".into(),
            alpha_acid_pct: aa,
            boil_time_minutes: minutes,
            form: Some("pellet".into()),
            r#use: Some("boil".into()),
            inventory_lot_id: None,
        }
    }

    #[test]
    fn compute_calcs_matches_golden_fixture() {
        let rec = Recipe {
            id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            name: "Golden".into(),
            r#type: "all_grain".into(),
            style_id: None,
            equipment_profile_id: None,
            mash_profile_id: None,
            batch_size_liters: 20.0,
            boil_size_liters: None,
            boil_time_minutes: None,
            efficiency_pct: Some(72.0),
            calc_og: None,
            calc_fg: None,
            calc_abv_pct: None,
            calc_ibu: None,
            calc_color_ebc: None,
            tasting_aroma: None,
            tasting_flavour: None,
            tasting_mouthfeel: None,
            tasting_finish: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let ferms = vec![ferm(1, 4.5, 37.0, 6.0), ferm(2, 0.5, 34.0, 120.0)];
        let hops = vec![hop(1, 25.0, 11.0, 60.0), hop(2, 20.0, 11.0, 10.0)];
        let yeasts = vec![Yeast {
            id: Uuid::nil(),
            recipe_id: Uuid::nil(),
            yeast_id: None,
            name: "Y".into(),
            amount: 11.0,
            unit: "g".into(),
            attenuation_pct: Some(78.0),
            inventory_lot_id: None,
        }];

        let c = compute_calcs(&rec, &ferms, &hops, &yeasts);
        assert!((c.calc_og - GOLDEN_OG).abs() < 1e-9, "og {}", c.calc_og);
        assert!((c.calc_fg - GOLDEN_FG).abs() < 1e-9, "fg {}", c.calc_fg);
        assert!(
            (c.calc_abv_pct - GOLDEN_ABV).abs() < 1e-9,
            "abv {}",
            c.calc_abv_pct
        );
        assert!((c.calc_ibu - GOLDEN_IBU).abs() < 1e-9, "ibu {}", c.calc_ibu);
        assert!(
            (c.calc_color_ebc - GOLDEN_EBC).abs() < 1e-9,
            "ebc {}",
            c.calc_color_ebc
        );
    }
}
