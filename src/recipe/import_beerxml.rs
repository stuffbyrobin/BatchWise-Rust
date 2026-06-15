//! BeerXML 1.0 recipe import.
//!
//! Port of the Go `internal/recipe/beerxml.go`. The input payload is a
//! base64-encoded BeerXML 1.0 document; it is decoded, parsed with
//! `quick-xml`/serde, and the first `RECIPE` element is mapped onto a
//! [`CreateRequest`]. Unit conversions and type mappings mirror the Go parser
//! exactly:
//!
//! * fermentable `AMOUNT` is kilograms (kept as kg),
//! * hop and yeast `AMOUNT` are kilograms and converted to grams,
//! * `POTENTIAL` (SG, e.g. 1.037) becomes PPG via `(potential - 1) * 1000`,
//! * fermentable `COLOR` is Lovibond and converted to EBC via `* 2.65 * 1.97`.

use crate::recipe::models::{CreateRequest, FermentableInput, HopInput, MashStepInput, YeastInput};
use serde::Deserialize;

// ---- BeerXML element structs ----
//
// BeerXML element names are UPPERCASE. The document root in practice is
// `<RECIPES>` wrapping one or more `<RECIPE>` elements; quick-xml deserializes
// from the root element, so the top-level struct maps the `RECIPE` children.

#[derive(Debug, Deserialize)]
struct BeerxmlRecipes {
    #[serde(rename = "RECIPE", default)]
    recipes: Vec<BeerxmlRecipe>,
}

#[derive(Debug, Deserialize)]
struct BeerxmlRecipe {
    #[serde(rename = "NAME", default)]
    name: String,
    #[serde(rename = "TYPE", default)]
    r#type: String,
    #[serde(rename = "BATCH_SIZE", default)]
    batch_size: f64,
    #[serde(rename = "BOIL_SIZE", default)]
    boil_size: f64,
    #[serde(rename = "BOIL_TIME", default)]
    boil_time: f64,
    #[serde(rename = "EFFICIENCY", default)]
    efficiency: f64,
    #[serde(rename = "NOTES", default)]
    notes: String,
    #[serde(rename = "FERMENTABLES", default)]
    fermentables: Option<BeerxmlFermentables>,
    #[serde(rename = "HOPS", default)]
    hops: Option<BeerxmlHops>,
    #[serde(rename = "YEASTS", default)]
    yeasts: Option<BeerxmlYeasts>,
    #[serde(rename = "MASH", default)]
    mash: Option<BeerxmlMash>,
}

#[derive(Debug, Deserialize)]
struct BeerxmlFermentables {
    #[serde(rename = "FERMENTABLE", default)]
    items: Vec<BeerxmlFerment>,
}

#[derive(Debug, Deserialize)]
struct BeerxmlFerment {
    #[serde(rename = "NAME", default)]
    name: String,
    #[serde(rename = "AMOUNT", default)]
    amount: f64, // kg
    #[serde(rename = "TYPE", default)]
    r#type: String,
    #[serde(rename = "COLOR", default)]
    color: f64, // Lovibond
    #[serde(rename = "POTENTIAL", default)]
    potential: f64, // SG e.g. 1.037
}

#[derive(Debug, Deserialize)]
struct BeerxmlHops {
    #[serde(rename = "HOP", default)]
    items: Vec<BeerxmlHop>,
}

#[derive(Debug, Deserialize)]
struct BeerxmlHop {
    #[serde(rename = "NAME", default)]
    name: String,
    #[serde(rename = "AMOUNT", default)]
    amount: f64, // kg
    #[serde(rename = "USE", default)]
    r#use: String,
    #[serde(rename = "TIME", default)]
    time: f64, // minutes
    #[serde(rename = "ALPHA", default)]
    alpha: f64, // %
    #[serde(rename = "FORM", default)]
    form: String,
}

#[derive(Debug, Deserialize)]
struct BeerxmlYeasts {
    #[serde(rename = "YEAST", default)]
    items: Vec<BeerxmlYeast>,
}

#[derive(Debug, Deserialize)]
struct BeerxmlYeast {
    #[serde(rename = "NAME", default)]
    name: String,
    #[serde(rename = "AMOUNT", default)]
    amount: f64, // kg or L
    #[serde(rename = "ATTENUATION", default)]
    attenuation: f64, // %
}

#[derive(Debug, Deserialize)]
struct BeerxmlMash {
    #[serde(rename = "MASH_STEPS", default)]
    steps: Option<BeerxmlMashSteps>,
}

#[derive(Debug, Deserialize)]
struct BeerxmlMashSteps {
    #[serde(rename = "MASH_STEP", default)]
    items: Vec<BeerxmlMashStep>,
}

#[derive(Debug, Deserialize)]
struct BeerxmlMashStep {
    #[serde(rename = "TYPE", default)]
    r#type: String,
    #[serde(rename = "STEP_TEMP", default)]
    step_temp: f64,
    #[serde(rename = "STEP_TIME", default)]
    step_time: f64,
    #[serde(rename = "INFUSE_AMOUNT", default)]
    infuse_amount: f64,
}

/// Decode a base64-encoded BeerXML 1.0 payload into a [`CreateRequest`].
pub fn parse_beerxml(data: &str) -> Result<CreateRequest, String> {
    let raw = base64_decode(data).map_err(|e| format!("beerxml: base64 decode: {e}"))?;
    let xml = String::from_utf8(raw).map_err(|e| format!("beerxml: utf8 decode: {e}"))?;

    let root: BeerxmlRecipes =
        quick_xml::de::from_str(&xml).map_err(|e| format!("beerxml: xml parse: {e}"))?;

    let r = root
        .recipes
        .into_iter()
        .next()
        .ok_or_else(|| "beerxml: no RECIPE elements found".to_string())?;

    let fermentables = r.fermentables.map(|f| f.items).unwrap_or_default();
    let hops = r.hops.map(|h| h.items).unwrap_or_default();
    let yeasts = r.yeasts.map(|y| y.items).unwrap_or_default();
    let mash_steps = r
        .mash
        .and_then(|m| m.steps)
        .map(|s| s.items)
        .unwrap_or_default();

    if fermentables.is_empty() {
        return Err("beerxml: no FERMENTABLE elements".to_string());
    }
    if yeasts.is_empty() {
        return Err("beerxml: no YEAST elements".to_string());
    }

    let ferment_inputs: Vec<FermentableInput> = fermentables
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let ppg = (f.potential - 1.0) * 1000.0;
            let color_ebc = f.color * 2.65 * 1.97; // Lovibond → SRM → EBC
            let typ = beerxml_fermentable_type(&f.r#type);
            FermentableInput {
                step_order: (i + 1) as i32,
                name: f.name,
                amount: f.amount,
                unit: "kg".to_string(),
                color_ebc: Some(color_ebc),
                potential_ppg: Some(ppg),
                r#type: Some(typ),
                addition: None,
            }
        })
        .collect();

    let hop_inputs: Vec<HopInput> = hops
        .into_iter()
        .enumerate()
        .map(|(i, h)| {
            let r#use = beerxml_hop_use(&h.r#use);
            let form = beerxml_hop_form(&h.form);
            let amt_g = h.amount * 1000.0; // kg → g
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

    let yeast_inputs: Vec<YeastInput> = yeasts
        .into_iter()
        .map(|y| {
            let amt_g = y.amount * 1000.0; // assume kg; convert to g
            YeastInput {
                yeast_id: None,
                name: y.name,
                amount: amt_g,
                unit: "g".to_string(),
                attenuation_pct: Some(y.attenuation),
            }
        })
        .collect();

    let mash_inputs: Vec<MashStepInput> = mash_steps
        .into_iter()
        .enumerate()
        .map(|(i, ms)| {
            let step_type = beerxml_mash_step_type(&ms.r#type);
            let infusion_volume_liters = if ms.infuse_amount > 0.0 {
                Some(ms.infuse_amount)
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
        r#type: beerxml_recipe_type(&r.r#type),
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

fn beerxml_recipe_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "all grain" => "all_grain",
        "extract" => "extract",
        "partial mash" => "partial_mash",
        _ => "other",
    }
    .to_string()
}

fn beerxml_fermentable_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "grain" | "adjunct" => "base",
        "crystal" | "caramel" => "crystal",
        "roasted" => "roasted",
        _ => "specialty",
    }
    .to_string()
}

fn beerxml_hop_use(use_: &str) -> String {
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

fn beerxml_hop_form(form: &str) -> String {
    match form.to_lowercase().as_str() {
        "leaf" | "whole" => "leaf",
        "extract" => "extract",
        _ => "pellet",
    }
    .to_string()
}

fn beerxml_mash_step_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "temperature" => "temperature",
        "decoction" => "decoction",
        _ => "infusion",
    }
    .to_string()
}

/// Minimal standard (RFC 4648) base64 decoder. Mirrors Go's
/// `base64.StdEncoding.DecodeString`: requires `=` padding and rejects any
/// character outside the standard alphabet (whitespace included).
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("illegal base64 character {c:#x}")),
        }
    }

    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err("invalid base64 length".to_string());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 {
            return Err("invalid base64 padding".to_string());
        }
        let b0 = val(chunk[0])?;
        let b1 = val(chunk[1])?;
        let n = (b0 as u32) << 18 | (b1 as u32) << 12;
        match pad {
            0 => {
                let b2 = val(chunk[2])?;
                let b3 = val(chunk[3])?;
                let n = n | (b2 as u32) << 6 | b3 as u32;
                out.push((n >> 16) as u8);
                out.push((n >> 8) as u8);
                out.push(n as u8);
            }
            1 => {
                let b2 = val(chunk[2])?;
                let n = n | (b2 as u32) << 6;
                out.push((n >> 16) as u8);
                out.push((n >> 8) as u8);
            }
            2 => {
                out.push((n >> 16) as u8);
            }
            _ => unreachable!(),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inlined copy of internal/recipe/testdata/sample.xml (the Go testdata
    // lives in the Go repo only, so it is embedded here directly).
    const SAMPLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<RECIPES>
  <RECIPE>
    <NAME>Sample IPA</NAME>
    <VERSION>1</VERSION>
    <TYPE>All Grain</TYPE>
    <BATCH_SIZE>19.0</BATCH_SIZE>
    <BOIL_SIZE>23.5</BOIL_SIZE>
    <BOIL_TIME>60</BOIL_TIME>
    <EFFICIENCY>72</EFFICIENCY>
    <NOTES>A classic American IPA fixture for import tests.</NOTES>
    <FERMENTABLES>
      <FERMENTABLE>
        <NAME>Pale Malt (2 Row) US</NAME>
        <VERSION>1</VERSION>
        <AMOUNT>4.5</AMOUNT>
        <TYPE>Grain</TYPE>
        <YIELD>78.7</YIELD>
        <COLOR>3.93</COLOR>
        <POTENTIAL>1.036</POTENTIAL>
      </FERMENTABLE>
      <FERMENTABLE>
        <NAME>Crystal 40L</NAME>
        <VERSION>1</VERSION>
        <AMOUNT>0.25</AMOUNT>
        <TYPE>Crystal</TYPE>
        <YIELD>74.1</YIELD>
        <COLOR>105.6</COLOR>
        <POTENTIAL>1.034</POTENTIAL>
      </FERMENTABLE>
    </FERMENTABLES>
    <HOPS>
      <HOP>
        <NAME>Citra</NAME>
        <VERSION>1</VERSION>
        <AMOUNT>0.030</AMOUNT>
        <USE>Boil</USE>
        <TIME>60</TIME>
        <ALPHA>12.0</ALPHA>
        <FORM>Pellet</FORM>
      </HOP>
      <HOP>
        <NAME>Citra</NAME>
        <VERSION>1</VERSION>
        <AMOUNT>0.030</AMOUNT>
        <USE>Boil</USE>
        <TIME>10</TIME>
        <ALPHA>12.0</ALPHA>
        <FORM>Pellet</FORM>
      </HOP>
      <HOP>
        <NAME>Citra</NAME>
        <VERSION>1</VERSION>
        <AMOUNT>0.050</AMOUNT>
        <USE>Dry Hop</USE>
        <TIME>0</TIME>
        <ALPHA>12.0</ALPHA>
        <FORM>Pellet</FORM>
      </HOP>
    </HOPS>
    <YEASTS>
      <YEAST>
        <NAME>Safale US-05</NAME>
        <VERSION>1</VERSION>
        <AMOUNT>0.0115</AMOUNT>
        <AMOUNT_IS_WEIGHT>TRUE</AMOUNT_IS_WEIGHT>
        <ATTENUATION>78</ATTENUATION>
        <TYPE>Ale</TYPE>
        <FORM>Dry</FORM>
      </YEAST>
    </YEASTS>
    <MASH>
      <NAME>Single Infusion</NAME>
      <VERSION>1</VERSION>
      <MASH_STEPS>
        <MASH_STEP>
          <NAME>Saccharification</NAME>
          <VERSION>1</VERSION>
          <TYPE>Infusion</TYPE>
          <STEP_TEMP>67</STEP_TEMP>
          <STEP_TIME>60</STEP_TIME>
          <INFUSE_AMOUNT>18.0</INFUSE_AMOUNT>
        </MASH_STEP>
        <MASH_STEP>
          <NAME>Mash Out</NAME>
          <VERSION>1</VERSION>
          <TYPE>Temperature</TYPE>
          <STEP_TEMP>77</STEP_TEMP>
          <STEP_TIME>10</STEP_TIME>
        </MASH_STEP>
      </MASH_STEPS>
    </MASH>
  </RECIPE>
</RECIPES>
"#;

    fn base64_encode_std(input: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = *chunk.get(1).unwrap_or(&0) as u32;
            let b2 = *chunk.get(2).unwrap_or(&0) as u32;
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALPHABET[(n >> 18 & 63) as usize] as char);
            out.push(ALPHABET[(n >> 12 & 63) as usize] as char);
            if chunk.len() > 1 {
                out.push(ALPHABET[(n >> 6 & 63) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(ALPHABET[(n & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }

    #[test]
    fn parses_sample_beerxml() {
        let encoded = base64_encode_std(SAMPLE_XML.as_bytes());
        let req = parse_beerxml(&encoded).expect("should parse sample BeerXML");

        assert_eq!(req.name, "Sample IPA");
        assert_eq!(req.r#type, "all_grain");
        assert_eq!(req.batch_size_liters, 19.0);

        let ferms = req.fermentables.as_ref().expect("fermentables");
        assert!(!ferms.is_empty());
        assert_eq!(ferms.len(), 2);
        assert_eq!(ferms[0].step_order, 1);
        assert_eq!(ferms[0].unit, "kg");
        assert_eq!(ferms[0].amount, 4.5);

        let hops = req.hops.as_ref().expect("hops");
        assert!(!hops.is_empty());
        assert_eq!(hops.len(), 3);
        // kg → g conversion: 0.030 kg → 30 g
        assert!((hops[0].amount - 30.0).abs() < 1e-9);
        assert_eq!(hops[0].unit, "g");
        assert_eq!(hops[2].r#use.as_deref(), Some("dry-hop"));

        let yeasts = req.yeasts.as_ref().expect("yeasts");
        assert_eq!(yeasts.len(), 1);
        assert_eq!(yeasts[0].unit, "g");

        let mash = req.mash_steps.as_ref().expect("mash steps");
        assert_eq!(mash.len(), 2);
        assert_eq!(mash[0].step_type, "infusion");
        assert_eq!(mash[0].infusion_volume_liters, Some(18.0));
        assert_eq!(mash[1].step_type, "temperature");
        assert_eq!(mash[1].infusion_volume_liters, None);
    }

    #[test]
    fn rejects_invalid_base64() {
        let err = parse_beerxml("not-base64!!!").unwrap_err();
        assert!(err.contains("base64"));
    }

    #[test]
    fn rejects_missing_recipe() {
        let encoded = base64_encode_std(b"<RECIPES></RECIPES>");
        let err = parse_beerxml(&encoded).unwrap_err();
        assert!(err.contains("no RECIPE"));
    }
}
