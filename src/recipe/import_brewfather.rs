//! Brewfather JSON recipe import.
//!
//! Port of the Go `internal/recipe/brewfather.go`. The input is a raw Brewfather
//! JSON export string (not base64-encoded). It is parsed with `serde_json` and
//! mapped onto a [`CreateRequest`]. Unit conversions and type mappings mirror
//! the Go parser exactly:
//!
//! * fermentable amounts are normalised to kg (a `g` unit is divided by 1000),
//! * hop and yeast amounts are normalised to g (a `kg` unit is multiplied by 1000),
//! * `potential` (SG, e.g. 1.037) becomes PPG via `(potential - 1) * 1000`,
//! * fermentable `color` is already EBC and kept as-is.

use crate::recipe::models::{CreateRequest, FermentableInput, HopInput, MashStepInput, YeastInput};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct BfRecipe {
    #[serde(default)]
    name: String,
    #[serde(default)]
    r#type: String,
    #[serde(rename = "batchSize", default)]
    batch_size: f64,
    #[serde(rename = "boilSize", default)]
    boil_size: f64,
    #[serde(rename = "boilTime", default)]
    boil_time: f64,
    #[serde(default)]
    efficiency: f64,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    fermentables: Vec<BfFermentable>,
    #[serde(default)]
    hops: Vec<BfHop>,
    #[serde(default)]
    yeasts: Vec<BfYeast>,
    #[serde(default)]
    mash: BfMash,
}

#[derive(Debug, Deserialize)]
struct BfFermentable {
    #[serde(default)]
    name: String,
    #[serde(default)]
    amount: f64, // kg (unless unit says otherwise)
    #[serde(default)]
    unit: String,
    #[serde(default)]
    color: f64, // EBC
    #[serde(default)]
    potential: f64, // SG e.g. 1.037
    #[serde(default)]
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct BfHop {
    #[serde(default)]
    name: String,
    #[serde(default)]
    amount: f64, // g (unless unit says otherwise)
    #[serde(default)]
    unit: String,
    #[serde(default)]
    r#use: String,
    #[serde(default)]
    time: f64, // minutes
    #[serde(default)]
    alpha: f64, // %
    #[serde(default)]
    form: String,
}

#[derive(Debug, Deserialize)]
struct BfYeast {
    #[serde(default)]
    name: String,
    #[serde(default)]
    amount: f64, // g (unless unit says otherwise)
    #[serde(default)]
    unit: String,
    #[serde(default)]
    attenuation: f64, // %
}

#[derive(Debug, Default, Deserialize)]
struct BfMash {
    #[serde(default)]
    steps: Vec<BfMashStep>,
}

#[derive(Debug, Deserialize)]
struct BfMashStep {
    #[serde(default)]
    r#type: String,
    #[serde(rename = "stepTemp", default)]
    step_temp: f64,
    #[serde(rename = "stepTime", default)]
    step_time: f64,
    #[serde(default)]
    amount: f64, // infusion volume L
}

/// Decode a Brewfather JSON export string into a [`CreateRequest`].
pub fn parse_brewfather(data: &str) -> Result<CreateRequest, String> {
    let r: BfRecipe =
        serde_json::from_str(data).map_err(|e| format!("brewfather: json parse: {e}"))?;

    if r.name.is_empty() {
        return Err("brewfather: missing required field 'name'".to_string());
    }
    if r.batch_size <= 0.0 {
        return Err("brewfather: missing required field 'batchSize'".to_string());
    }
    if r.fermentables.is_empty() {
        return Err("brewfather: no fermentables".to_string());
    }
    if r.yeasts.is_empty() {
        return Err("brewfather: no yeasts".to_string());
    }

    let ferment_inputs: Vec<FermentableInput> = r
        .fermentables
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            // Brewfather uses EBC; convert potential SG → PPG.
            let ppg = (f.potential - 1.0) * 1000.0;
            let color_ebc = f.color;

            // Normalise amount to kg.
            let amt_kg = if f.unit.to_lowercase() == "g" {
                f.amount / 1000.0
            } else {
                f.amount
            };

            FermentableInput {
                step_order: (i + 1) as i32,
                name: f.name,
                amount: amt_kg,
                unit: "kg".to_string(),
                color_ebc: Some(color_ebc),
                potential_ppg: Some(ppg),
                r#type: Some(bf_fermentable_type(&f.r#type)),
                addition: None,
            }
        })
        .collect();

    let hop_inputs: Vec<HopInput> = r
        .hops
        .into_iter()
        .enumerate()
        .map(|(i, h)| {
            let r#use = bf_hop_use(&h.r#use);
            let form = bf_hop_form(&h.form);

            // Normalise amount to g.
            let amt_g = if h.unit.to_lowercase() == "kg" {
                h.amount * 1000.0
            } else {
                h.amount
            };

            HopInput {
                step_order: (i + 1) as i32,
                name: h.name,
                amount: amt_g,
                unit: "g".to_string(),
                alpha_acid_pct: h.alpha,
                boil_time_minutes: h.time,
                form: Some(form),
                r#use: Some(r#use),
            }
        })
        .collect();

    let yeast_inputs: Vec<YeastInput> = r
        .yeasts
        .into_iter()
        .map(|y| {
            let amt_g = if y.unit.to_lowercase() == "kg" {
                y.amount * 1000.0
            } else {
                y.amount
            };
            YeastInput {
                yeast_id: None,
                name: y.name,
                amount: amt_g,
                unit: "g".to_string(),
                attenuation_pct: Some(y.attenuation),
            }
        })
        .collect();

    let mash_inputs: Vec<MashStepInput> = r
        .mash
        .steps
        .into_iter()
        .enumerate()
        .map(|(i, ms)| {
            let step_type = bf_mash_step_type(&ms.r#type);
            let infusion_volume_liters = if ms.amount > 0.0 && step_type == "infusion" {
                Some(ms.amount)
            } else {
                None
            };
            MashStepInput {
                step_order: (i + 1) as i32,
                step_type,
                target_temp_c: ms.step_temp,
                hold_minutes: ms.step_time as i32,
                infusion_volume_liters,
            }
        })
        .collect();

    Ok(CreateRequest {
        name: r.name,
        r#type: bf_recipe_type(&r.r#type),
        style_id: None,
        equipment_profile_id: None,
        mash_profile_id: None,
        batch_size_liters: r.batch_size,
        boil_size_liters: if r.boil_size > 0.0 {
            Some(r.boil_size)
        } else {
            None
        },
        boil_time_minutes: if r.boil_time > 0.0 {
            Some(r.boil_time as i32)
        } else {
            None
        },
        efficiency_pct: if r.efficiency > 0.0 {
            Some(r.efficiency)
        } else {
            None
        },
        tasting_aroma: None,
        tasting_flavour: None,
        tasting_mouthfeel: None,
        tasting_finish: None,
        notes: if r.notes.is_empty() {
            None
        } else {
            Some(r.notes)
        },
        fermentables: Some(ferment_inputs),
        hops: Some(hop_inputs),
        yeasts: Some(yeast_inputs),
        mash_steps: Some(mash_inputs),
    })
}

fn bf_recipe_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "all grain" => "all_grain",
        "extract" => "extract",
        "partial mash" => "partial_mash",
        _ => "other",
    }
    .to_string()
}

fn bf_fermentable_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "grain" | "adjunct" => "base",
        "crystal" | "caramel" => "crystal",
        "roasted" => "roasted",
        _ => "specialty",
    }
    .to_string()
}

fn bf_hop_use(use_: &str) -> String {
    match use_.to_lowercase().as_str() {
        "boil" => "boil",
        "dry hop" => "dry-hop",
        "whirlpool" => "whirlpool",
        "first wort" => "first-wort",
        "mash" => "mash",
        _ => "boil",
    }
    .to_string()
}

fn bf_hop_form(form: &str) -> String {
    match form.to_lowercase().as_str() {
        "leaf" | "whole" => "leaf",
        "extract" => "extract",
        _ => "pellet",
    }
    .to_string()
}

fn bf_mash_step_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "temperature" => "temperature",
        "decoction" => "decoction",
        _ => "infusion",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inlined copy of internal/recipe/testdata/sample_brewfather.json (the Go
    // testdata lives in the Go repo only, so it is embedded here directly).
    const SAMPLE_JSON: &str = r#"{
  "name": "Sample Pale Ale BF",
  "type": "All Grain",
  "batchSize": 20.0,
  "boilSize": 25.0,
  "boilTime": 60,
  "efficiency": 75,
  "notes": "Brewfather fixture for import tests.",
  "fermentables": [
    {
      "name": "Pale Malt (Maris Otter)",
      "amount": 4.0,
      "unit": "kg",
      "color": 5.9,
      "potential": 1.037,
      "type": "Grain"
    },
    {
      "name": "Crystal 60",
      "amount": 0.3,
      "unit": "kg",
      "color": 157.8,
      "potential": 1.033,
      "type": "Crystal"
    }
  ],
  "hops": [
    {
      "name": "Cascade",
      "amount": 25,
      "unit": "g",
      "use": "Boil",
      "time": 60,
      "alpha": 5.5,
      "form": "Pellet"
    },
    {
      "name": "Cascade",
      "amount": 15,
      "unit": "g",
      "use": "Boil",
      "time": 15,
      "alpha": 5.5,
      "form": "Pellet"
    },
    {
      "name": "Cascade",
      "amount": 30,
      "unit": "g",
      "use": "Dry Hop",
      "time": 0,
      "alpha": 5.5,
      "form": "Pellet"
    }
  ],
  "yeasts": [
    {
      "name": "Safale S-04",
      "amount": 11.5,
      "unit": "g",
      "attenuation": 72,
      "type": "Ale",
      "form": "Dry"
    }
  ],
  "mash": {
    "steps": [
      {
        "name": "Saccharification",
        "type": "Infusion",
        "stepTemp": 66.0,
        "stepTime": 75,
        "amount": 12.0
      },
      {
        "name": "Mash Out",
        "type": "Temperature",
        "stepTemp": 76.0,
        "stepTime": 10
      }
    ]
  }
}
"#;

    #[test]
    fn parses_sample_brewfather() {
        let req = parse_brewfather(SAMPLE_JSON).expect("should parse sample Brewfather JSON");

        assert_eq!(req.name, "Sample Pale Ale BF");
        assert_eq!(req.r#type, "all_grain");
        assert_eq!(req.batch_size_liters, 20.0);

        let ferms = req.fermentables.as_ref().expect("fermentables");
        assert!(!ferms.is_empty());
        assert_eq!(ferms.len(), 2);
        assert_eq!(ferms[0].step_order, 1);
        assert_eq!(ferms[0].unit, "kg");
        assert_eq!(ferms[0].amount, 4.0);
        assert_eq!(ferms[0].color_ebc, Some(5.9));

        let hops = req.hops.as_ref().expect("hops");
        assert!(!hops.is_empty());
        assert_eq!(hops.len(), 3);
        assert_eq!(hops[0].unit, "g");
        assert_eq!(hops[0].amount, 25.0); // already g, unchanged
        assert_eq!(hops[2].r#use.as_deref(), Some("dry-hop"));

        let yeasts = req.yeasts.as_ref().expect("yeasts");
        assert_eq!(yeasts.len(), 1);
        assert_eq!(yeasts[0].unit, "g");
        assert_eq!(yeasts[0].amount, 11.5);

        let mash = req.mash_steps.as_ref().expect("mash steps");
        assert_eq!(mash.len(), 2);
        assert_eq!(mash[0].step_type, "infusion");
        assert_eq!(mash[0].infusion_volume_liters, Some(12.0));
        assert_eq!(mash[1].step_type, "temperature");
        assert_eq!(mash[1].infusion_volume_liters, None);
    }

    #[test]
    fn rejects_invalid_json() {
        let err = parse_brewfather("{not valid json").unwrap_err();
        assert!(err.contains("json parse"));
    }

    #[test]
    fn rejects_missing_name() {
        let err =
            parse_brewfather(r#"{"batchSize": 20, "fermentables": [], "yeasts": []}"#).unwrap_err();
        assert!(err.contains("name"));
    }
}
