//! Brewing water chemistry calculations.
//!
//! Port of the Go `pkg/water` package. Fallible operations return
//! [`Result<T, WaterError>`] in place of Go's `(T, error)` tuples.

use std::collections::HashMap;
use std::fmt;

// Constants from §6.2, §6.3, §6.4 of PHYSICS_REFERENCE.md.
pub const BICARBONATE_TO_CACO3: f64 = 50.0 / 61.0;
pub const CACO3_TO_BICARBONATE: f64 = 61.0 / 50.0;

pub const CALCIUM_RA_FACTOR: f64 = 0.7143;
pub const MAGNESIUM_RA_FACTOR: f64 = 0.5879;

pub const MW_CALCIUM_CARBONATE: f64 = 100.09;
// For salts that come in more than one form, `MW_<salt>` holds the form used by
// default and `MW_<salt>_<form>` the alternative. `form_molar_mass` picks one.
pub const MW_CALCIUM_SULFATE: f64 = 172.17; // CaSO4·2H2O (gypsum, dihydrate — default)
pub const MW_CALCIUM_SULFATE_ANHYDROUS: f64 = 136.14; // CaSO4 (anhydrous)
pub const MW_CALCIUM_CHLORIDE: f64 = 147.01; // CaCl2·2H2O (dihydrate — default)
pub const MW_CALCIUM_CHLORIDE_ANHYDROUS: f64 = 110.98; // CaCl2 (anhydrous / dissolved basis)
pub const MW_MAGNESIUM_SULFATE: f64 = 246.47; // MgSO4·7H2O (Epsom, heptahydrate — default)
pub const MW_MAGNESIUM_SULFATE_ANHYDROUS: f64 = 120.37; // MgSO4 (anhydrous)
pub const MW_MAGNESIUM_CHLORIDE: f64 = 203.30; // MgCl2·6H2O (hexahydrate — default)
pub const MW_MAGNESIUM_CHLORIDE_ANHYDROUS: f64 = 95.21; // MgCl2 (anhydrous)
pub const MW_SODIUM_BICARBONATE: f64 = 84.01;
pub const MW_SODIUM_CHLORIDE: f64 = 58.44;
pub const MW_SODIUM_SULFATE: f64 = 142.04; // Na2SO4 (anhydrous — default)
pub const MW_SODIUM_SULFATE_DECAHYDRATE: f64 = 322.20; // Na2SO4·10H2O (Glauber's salt)
pub const MW_CALCIUM_HYDROXIDE: f64 = 74.09;
pub const MW_CALCIUM: f64 = 40.08;
pub const MW_MAGNESIUM: f64 = 24.31;
pub const MW_SODIUM: f64 = 22.99;
pub const MW_SULFATE: f64 = 96.06;
pub const MW_CHLORIDE: f64 = 35.45;
pub const MW_BICARBONATE: f64 = 61.02;
pub const MW_CARBONATE: f64 = 60.01;

pub const BASELINE_PHOSPHATE_ALKALINITY: f64 = 5.75;
pub const PHOSPHATE_SENSITIVITY: f64 = 0.17;

/// Errors returned by the water chemistry calculations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaterError {
    /// An input failed a precondition (the message describes which).
    Invalid(String),
}

impl fmt::Display for WaterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaterError::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for WaterError {}

/// Holds ion concentrations in ppm (mg/L).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WaterProfile {
    /// Ca²⁺ ppm.
    pub calcium: f64,
    /// Mg²⁺ ppm.
    pub magnesium: f64,
    /// Na⁺ ppm.
    pub sodium: f64,
    /// SO₄²⁻ ppm.
    pub sulfate: f64,
    /// Cl⁻ ppm.
    pub chloride: f64,
    /// HCO₃⁻ ppm.
    pub bicarbonate: f64,
}

/// Identifies a mineral salt addition.
///
/// `Gypsum` (CaSO4), `CalciumCl` (CaCl2), `Chalk` (CaCO3), `Epsom` (MgSO4),
/// `MagnesiumCl` (MgCl2), `BakingSoda` (NaHCO3), `TableSalt` (NaCl),
/// `SodiumSulfate` (Na2SO4), `SlakedLime` (Ca(OH)₂).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MineralType {
    /// CaSO4.
    Gypsum,
    /// CaCl2.
    CalciumCl,
    /// CaCO3.
    Chalk,
    /// MgSO4.
    Epsom,
    /// MgCl2.
    MagnesiumCl,
    /// NaHCO3.
    BakingSoda,
    /// NaCl.
    TableSalt,
    /// Na2SO4.
    SodiumSulfate,
    /// Ca(OH)₂.
    SlakedLime,
}

/// The physical form a salt is supplied in, which changes how a given weight
/// maps to dissolved ions. The specific molar mass per `(salt, form)` is
/// resolved by [`form_molar_mass`].
///
/// * `Anhydrous` — the bare salt.
/// * `Hydrate` — the standard hydrated crystal brewers weigh (e.g. CaCl2·2H2O,
///   MgSO4·7H2O, MgCl2·6H2O, Na2SO4·10H2O).
/// * `Liquid` — an aqueous solution; `strength_pct` (%w/w) gives the anhydrous
///   salt fraction of the supplied weight. Only meaningful for CaCl2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MineralForm {
    Anhydrous,
    #[default]
    Hydrate,
    Liquid,
}

/// Molar mass (g/mol) of `mineral_type` in the supplied `form`. Salts without a
/// hydrate distinction return their single molar mass regardless of `form`.
/// `Liquid` resolves to the anhydrous mass (the dissolved-salt basis).
pub fn form_molar_mass(mineral_type: MineralType, form: MineralForm) -> f64 {
    use MineralForm::{Anhydrous, Liquid};
    use MineralType::*;
    let anhydrous = matches!(form, Anhydrous | Liquid);
    match mineral_type {
        Gypsum if anhydrous => MW_CALCIUM_SULFATE_ANHYDROUS,
        Gypsum => MW_CALCIUM_SULFATE,
        CalciumCl if anhydrous => MW_CALCIUM_CHLORIDE_ANHYDROUS,
        CalciumCl => MW_CALCIUM_CHLORIDE,
        Epsom if anhydrous => MW_MAGNESIUM_SULFATE_ANHYDROUS,
        Epsom => MW_MAGNESIUM_SULFATE,
        MagnesiumCl if anhydrous => MW_MAGNESIUM_CHLORIDE_ANHYDROUS,
        MagnesiumCl => MW_MAGNESIUM_CHLORIDE,
        // Na2SO4 default is anhydrous; the hydrate is Glauber's decahydrate.
        SodiumSulfate if anhydrous => MW_SODIUM_SULFATE,
        SodiumSulfate => MW_SODIUM_SULFATE_DECAHYDRATE,
        // No hydrate distinction.
        Chalk => MW_CALCIUM_CARBONATE,
        BakingSoda => MW_SODIUM_BICARBONATE,
        TableSalt => MW_SODIUM_CHLORIDE,
        SlakedLime => MW_CALCIUM_HYDROXIDE,
    }
}

/// Identifies an acid used for acidification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcidType {
    Phosphoric,
    Lactic,
    Sulfuric,
    Hydrochloric,
    /// Disables acid treatment.
    None,
}

/// Classifies malt for mash pH prediction.
///
/// `Base` is pale/pilsner; `Crystal` is caramel malt; `Roast` is
/// chocolate/black; `Acid` is Sauermalz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrainType {
    Base,
    Crystal,
    Roast,
    Acid,
}

/// Colour input unit for [`convert_colour_to_lovibond`].
///
/// `Lovibond` is degrees Lovibond; `Srm` is Standard Reference Method;
/// `Ebc` is European Brewery Convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourUnit {
    Lovibond,
    Srm,
    Ebc,
}

/// A single mineral salt to add to the water.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MineralAddition {
    pub mineral_type: MineralType,
    /// Grams. For a `Liquid` form this is the weight of solution; the anhydrous
    /// salt content is `amount * strength_pct / 100`.
    pub amount: f64,
    /// Supplied form (anhydrous/dihydrate/liquid). Only consulted for CaCl2;
    /// defaults to `Dihydrate` for all other salts and legacy data.
    pub form: MineralForm,
    /// Solution strength (%w/w), used only when `form` is `Liquid`.
    pub strength_pct: f64,
}

/// Describes an acid addition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcidAddition {
    pub acid_type: AcidType,
    /// % (e.g. 85.0 for 85%).
    pub strength: f64,
    /// mL.
    pub amount: f64,
}

/// Holds the ion and alkalinity changes caused by an acid addition.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AcidEffect {
    /// ppm as CaCO3.
    pub alk_reduction: f64,
    /// ppm SO4 added.
    pub sulfate: f64,
    /// ppm Cl added.
    pub chloride: f64,
    /// ppm PO4 added.
    pub phosphate: f64,
    /// ppm lactate added.
    pub lactate: f64,
    /// Total meq of acid.
    pub milli_equivs: f64,
}

/// A grain entry for mash pH prediction. `colour` is degrees Lovibond.
#[derive(Debug, Clone, PartialEq)]
pub struct GrainAddition {
    pub name: String,
    pub grain_type: GrainType,
    /// kg.
    pub weight: f64,
    /// °Lovibond.
    pub colour: f64,
}

/// Describes the mash for pH prediction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MashParameters {
    /// Litres.
    pub water_volume: f64,
    pub grains: Vec<GrainAddition>,
    pub acids: Vec<AcidAddition>,
}

/// Output of [`calculate_water_treatment`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalculatedResults {
    pub final_profile: WaterProfile,
    /// ppm as CaCO3.
    pub alkalinity: f64,
    /// Kolbach RA, ppm as CaCO3.
    pub residual_alk: f64,
    /// Balance ratio.
    pub sulfate_to_chloride: f64,
    /// 0 if no mash params given.
    pub mash_ph: f64,
}

/// Applies mineral and optional acid additions to `start`, then computes
/// alkalinity, RA, sulfate:chloride, and mash pH.
pub fn calculate_water_treatment(
    start: WaterProfile,
    vol: f64,
    mins: &[MineralAddition],
    mash: Option<&MashParameters>,
) -> Result<CalculatedResults, WaterError> {
    if vol <= 0.0 {
        return Err(WaterError::Invalid("volume must be > 0".to_string()));
    }
    let mut profile = start;
    for m in mins {
        apply_mineral(&mut profile, *m, vol)?;
    }
    if let Some(mash) = mash {
        for a in &mash.acids {
            let eff = calculate_acid_effect(*a, vol)?;
            let alk_caco3 = profile.bicarbonate * BICARBONATE_TO_CACO3;
            let alk_caco3 = (alk_caco3 - eff.alk_reduction).max(0.0);
            profile.bicarbonate = alk_caco3 * CACO3_TO_BICARBONATE;
            profile.sulfate += eff.sulfate;
            profile.chloride += eff.chloride;
        }
    }
    let alk = profile.bicarbonate * BICARBONATE_TO_CACO3;
    let ra = alk - CALCIUM_RA_FACTOR * profile.calcium - MAGNESIUM_RA_FACTOR * profile.magnesium;
    let mut sc = 0.0;
    if profile.chloride > 0.0 {
        sc = profile.sulfate / profile.chloride;
    }
    let mut ph = 0.0;
    if let Some(mash) = mash {
        if !mash.grains.is_empty() {
            ph = predict_mash_ph(profile, mash);
        }
    }
    Ok(CalculatedResults {
        final_profile: profile,
        alkalinity: alk,
        residual_alk: ra,
        sulfate_to_chloride: sc,
        mash_ph: ph,
    })
}

fn apply_mineral(p: &mut WaterProfile, m: MineralAddition, vol: f64) -> Result<(), WaterError> {
    // grams of salt dissolved in `vol` litres → ppm (mg/L): mass × 1000 / vol.
    // (Without the ×1000 the result is g/L, i.e. 1000× too small.)
    let ppm = 1000.0 / vol;
    // Molar mass of the chosen form. A liquid is dosed by solution weight, so
    // scale to the anhydrous salt it contains — `form_molar_mass` returns the
    // anhydrous mass for `Liquid`, so the two stay consistent.
    let mw = form_molar_mass(m.mineral_type, m.form);
    let g = if m.form == MineralForm::Liquid {
        m.amount * m.strength_pct / 100.0
    } else {
        m.amount
    };
    match m.mineral_type {
        MineralType::Gypsum => {
            p.calcium += g * MW_CALCIUM / mw * ppm;
            p.sulfate += g * MW_SULFATE / mw * ppm;
        }
        MineralType::CalciumCl => {
            p.calcium += g * MW_CALCIUM / mw * ppm;
            p.chloride += g * 2.0 * MW_CHLORIDE / mw * ppm;
        }
        MineralType::Chalk => {
            p.calcium += g * MW_CALCIUM / mw * ppm;
            p.bicarbonate += g * 2.0 * MW_BICARBONATE / mw * ppm;
        }
        MineralType::Epsom => {
            p.magnesium += g * MW_MAGNESIUM / mw * ppm;
            p.sulfate += g * MW_SULFATE / mw * ppm;
        }
        MineralType::MagnesiumCl => {
            p.magnesium += g * MW_MAGNESIUM / mw * ppm;
            p.chloride += g * 2.0 * MW_CHLORIDE / mw * ppm;
        }
        MineralType::BakingSoda => {
            p.sodium += g * MW_SODIUM / mw * ppm;
            p.bicarbonate += g * MW_BICARBONATE / mw * ppm;
        }
        MineralType::TableSalt => {
            p.sodium += g * MW_SODIUM / mw * ppm;
            p.chloride += g * MW_CHLORIDE / mw * ppm;
        }
        MineralType::SodiumSulfate => {
            p.sodium += g * 2.0 * MW_SODIUM / mw * ppm;
            p.sulfate += g * MW_SULFATE / mw * ppm;
        }
        MineralType::SlakedLime => {
            p.calcium += g * MW_CALCIUM / mw * ppm;
            // 2 OH- per mol Ca(OH)2 → 2 mol HCO3- equivalent
            p.bicarbonate += g * 2.0 * MW_BICARBONATE / mw * ppm;
        }
    }
    Ok(())
}

/// Returns the ion and alkalinity change from one acid addition in a given
/// volume of water.
pub fn calculate_acid_effect(acid: AcidAddition, vol: f64) -> Result<AcidEffect, WaterError> {
    if vol <= 0.0 {
        return Err(WaterError::Invalid("volume must be > 0".to_string()));
    }
    let (sg, mw, equiv) = acid_properties(acid.acid_type)?;
    // meq = (mL × SG × strength% × 10 × equivalents) / MW
    let meq = (acid.amount * sg * acid.strength * 10.0 * equiv as f64) / mw;
    let mut eff = AcidEffect {
        alk_reduction: (meq * 50.0) / vol,
        milli_equivs: meq,
        ..Default::default()
    };
    // Anion contributions: ppm = (mL × SG × strength × 10 × MW_anion) / (MW_acid × vol)
    let factor = acid.amount * sg * acid.strength * 10.0 / (mw * vol);
    match acid.acid_type {
        AcidType::Phosphoric => eff.phosphate = factor * 95.0, // PO4 MW ≈ 95
        AcidType::Lactic => eff.lactate = factor * 89.07,
        AcidType::Sulfuric => eff.sulfate = factor * MW_SULFATE,
        AcidType::Hydrochloric => eff.chloride = factor * MW_CHLORIDE,
        AcidType::None => {}
    }
    Ok(eff)
}

/// Returns `(sg, mw, equiv)` for the given acid type.
fn acid_properties(t: AcidType) -> Result<(f64, f64, i32), WaterError> {
    match t {
        AcidType::Phosphoric => Ok((1.685, 98.0, 3)),
        AcidType::Lactic => Ok((1.2, 90.08, 1)),
        AcidType::Sulfuric => Ok((1.84, 98.08, 2)),
        AcidType::Hydrochloric => Ok((1.18, 36.46, 1)),
        AcidType::None => Err(WaterError::Invalid(
            "unknown acid type: \"none\"".to_string(),
        )),
    }
}

/// Returns the mL of acid needed to acidify `sparge_vol` litres of
/// `sparge_water` to `target_ph`. Uses a simplified zero-alkalinity model.
pub fn calculate_sparge_acidification(
    sparge_water: WaterProfile,
    sparge_vol: f64,
    _target_ph: f64,
    acid_type: AcidType,
    acid_strength: f64,
) -> Result<f64, WaterError> {
    if sparge_vol <= 0.0 {
        return Err(WaterError::Invalid("spargeVol must be > 0".to_string()));
    }
    let (sg, mw, equiv) = acid_properties(acid_type)?;
    // target_ph is unused: the simplified model removes all alkalinity.
    let alk_caco3 = sparge_water.bicarbonate * BICARBONATE_TO_CACO3;
    let total_meq = alk_caco3 * sparge_vol / 50.0;
    // mL = meq × MW / (SG × strength × 10 × equiv)
    let ml = total_meq * mw / (sg * acid_strength * 10.0 * equiv as f64);
    Ok(ml)
}

/// Converts a colour value from the given unit to °Lovibond.
pub fn convert_colour_to_lovibond(colour: f64, unit: ColourUnit) -> f64 {
    match unit {
        ColourUnit::Lovibond => colour,
        ColourUnit::Srm => colour, // SRM ≈ Lovibond (industry approximation)
        ColourUnit::Ebc => colour / 2.65,
    }
}

/// Blends `w1` and `w2`. `ratio` = 0 returns `w1`; `ratio` = 1 returns `w2`.
pub fn blend_water_profiles(w1: WaterProfile, w2: WaterProfile, ratio: f64) -> WaterProfile {
    let lerp = |a: f64, b: f64| a * (1.0 - ratio) + b * ratio;
    WaterProfile {
        calcium: lerp(w1.calcium, w2.calcium),
        magnesium: lerp(w1.magnesium, w2.magnesium),
        sodium: lerp(w1.sodium, w2.sodium),
        sulfate: lerp(w1.sulfate, w2.sulfate),
        chloride: lerp(w1.chloride, w2.chloride),
        bicarbonate: lerp(w1.bicarbonate, w2.bicarbonate),
    }
}

/// Returns total dissolved solids as the sum of all ions in ppm.
pub fn calculate_tds(p: WaterProfile) -> f64 {
    p.calcium + p.magnesium + p.sodium + p.sulfate + p.chloride + p.bicarbonate
}

/// Returns total hardness as ppm CaCO3.
pub fn calculate_hardness(p: WaterProfile) -> f64 {
    p.calcium * 50.0 / 20.04 + p.magnesium * 50.0 / 12.155
}

/// Returns a concise string representation of a water profile.
pub fn format_water_profile(p: WaterProfile) -> String {
    format!(
        "Ca:{:.0} Mg:{:.0} Na:{:.0} SO4:{:.0} Cl:{:.0} HCO3:{:.0}",
        p.calcium, p.magnesium, p.sodium, p.sulfate, p.chloride, p.bicarbonate
    )
}

/// Returns well-known brewing water profiles (§6.8), keyed by lowercase name.
pub fn common_water_profiles() -> HashMap<&'static str, WaterProfile> {
    let mut m = HashMap::new();
    m.insert(
        "burton",
        WaterProfile {
            calcium: 295.0,
            magnesium: 45.0,
            sodium: 55.0,
            sulfate: 725.0,
            chloride: 25.0,
            bicarbonate: 300.0,
        },
    );
    m.insert(
        "dublin",
        WaterProfile {
            calcium: 115.0,
            magnesium: 4.0,
            sodium: 12.0,
            sulfate: 55.0,
            chloride: 19.0,
            bicarbonate: 319.0,
        },
    );
    m.insert(
        "dortmund",
        WaterProfile {
            calcium: 225.0,
            magnesium: 40.0,
            sodium: 60.0,
            sulfate: 120.0,
            chloride: 60.0,
            bicarbonate: 180.0,
        },
    );
    m.insert(
        "edinburgh",
        WaterProfile {
            calcium: 120.0,
            magnesium: 25.0,
            sodium: 55.0,
            sulfate: 140.0,
            chloride: 60.0,
            bicarbonate: 225.0,
        },
    );
    m.insert(
        "london",
        WaterProfile {
            calcium: 52.0,
            magnesium: 32.0,
            sodium: 86.0,
            sulfate: 32.0,
            chloride: 34.0,
            bicarbonate: 104.0,
        },
    );
    m.insert(
        "munich",
        WaterProfile {
            calcium: 75.0,
            magnesium: 18.0,
            sodium: 2.0,
            sulfate: 10.0,
            chloride: 2.0,
            bicarbonate: 150.0,
        },
    );
    m.insert(
        "pilsen",
        WaterProfile {
            calcium: 7.0,
            magnesium: 2.0,
            sodium: 2.0,
            sulfate: 5.0,
            chloride: 5.0,
            bicarbonate: 15.0,
        },
    );
    m.insert(
        "vienna",
        WaterProfile {
            calcium: 200.0,
            magnesium: 60.0,
            sodium: 8.0,
            sulfate: 125.0,
            chloride: 12.0,
            bicarbonate: 120.0,
        },
    );
    m.insert("ro", WaterProfile::default());
    m
}

/// Returns a named profile from [`common_water_profiles`] (case-insensitive).
pub fn get_water_profile(name: &str) -> Option<WaterProfile> {
    common_water_profiles()
        .get(name.to_lowercase().as_str())
        .copied()
}

/// Uses the deLange model to predict mash pH (§6.7).
fn predict_mash_ph(profile: WaterProfile, mash: &MashParameters) -> f64 {
    let alk_caco3 = profile.bicarbonate * BICARBONATE_TO_CACO3;
    let water_alk_meq = alk_caco3 / 50.0;

    let mut total_grain = 0.0;
    for g in &mash.grains {
        total_grain += g.weight;
    }
    if total_grain == 0.0 || mash.water_volume == 0.0 {
        return BASELINE_PHOSPHATE_ALKALINITY;
    }
    let ratio = mash.water_volume / total_grain; // L/kg

    let mut grain_acidity = 0.0;
    for g in &mash.grains {
        grain_acidity += grain_contrib(g, total_grain, ratio);
    }

    let ph =
        BASELINE_PHOSPHATE_ALKALINITY - PHOSPHATE_SENSITIVITY * (grain_acidity - water_alk_meq);
    ph.clamp(4.0, 7.0)
}

fn grain_contrib(g: &GrainAddition, total_grain: f64, ratio: f64) -> f64 {
    match g.grain_type {
        GrainType::Base => g.weight * (0.014 * g.colour - 0.034) / (total_grain * ratio),
        GrainType::Crystal => g.weight * (0.014 * g.colour + 0.45) / (total_grain * ratio),
        GrainType::Roast => g.weight * 0.60 / (total_grain * ratio),
        GrainType::Acid => {
            let pct = (g.weight / total_grain) * 100.0;
            g.weight * 0.07 * pct / (total_grain * ratio)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn alkalinity_conversions() {
        // 200 ppm HCO3 × 50/61 = 163.9 ppm CaCO3
        let alk = 200.0 * BICARBONATE_TO_CACO3;
        assert_relative_eq!(alk, 163.9, epsilon = 0.5);

        // round-trip
        let hco3 = alk * CACO3_TO_BICARBONATE;
        assert_relative_eq!(hco3, 200.0, epsilon = 0.1);
    }

    #[test]
    fn residual_alkalinity() {
        // Burton: Ca=295, Mg=45, HCO3=300
        let p = WaterProfile {
            calcium: 295.0,
            magnesium: 45.0,
            bicarbonate: 300.0,
            ..Default::default()
        };
        let alk = p.bicarbonate * BICARBONATE_TO_CACO3;
        let ra = alk - CALCIUM_RA_FACTOR * p.calcium - MAGNESIUM_RA_FACTOR * p.magnesium;
        // ~245 - 210.7 - 26.5 ≈ 7.8 (very low RA)
        assert_relative_eq!(ra, 7.8, epsilon = 5.0);
    }

    #[test]
    fn mineral_addition_gypsum() {
        let start = WaterProfile::default();
        let res = calculate_water_treatment(
            start,
            20.0,
            &[MineralAddition {
                mineral_type: MineralType::Gypsum,
                amount: 5.0,
                form: MineralForm::Hydrate,
                strength_pct: 0.0,
            }],
            None,
        )
        .unwrap();
        // 5g CaSO4·2H2O in 20L → ppm = mass × 1000 / 20
        assert_relative_eq!(
            res.final_profile.calcium,
            5.0 * 40.08 / 172.17 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert_relative_eq!(
            res.final_profile.sulfate,
            5.0 * 96.06 / 172.17 / 20.0 * 1000.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn mineral_addition_calcium_chloride() {
        let res = calculate_water_treatment(
            WaterProfile::default(),
            20.0,
            &[MineralAddition {
                mineral_type: MineralType::CalciumCl,
                amount: 5.0,
                form: MineralForm::Hydrate,
                strength_pct: 0.0,
            }],
            None,
        )
        .unwrap();
        assert_relative_eq!(
            res.final_profile.calcium,
            5.0 * 40.08 / 147.01 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert_relative_eq!(
            res.final_profile.chloride,
            5.0 * 2.0 * 35.45 / 147.01 / 20.0 * 1000.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn mineral_addition_calcium_chloride_forms() {
        let ca = |form: MineralForm, amount: f64, strength: f64| {
            calculate_water_treatment(
                WaterProfile::default(),
                20.0,
                &[MineralAddition {
                    mineral_type: MineralType::CalciumCl,
                    amount,
                    form,
                    strength_pct: strength,
                }],
                None,
            )
            .unwrap()
            .final_profile
            .calcium
        };
        // Anhydrous CaCl2 is more concentrated than the dihydrate crystal for the
        // same weight (no waters of hydration).
        let anhydrous = ca(MineralForm::Anhydrous, 5.0, 0.0);
        let dihydrate = ca(MineralForm::Hydrate, 5.0, 0.0);
        assert_relative_eq!(
            anhydrous,
            5.0 * 40.08 / 110.98 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert_relative_eq!(
            dihydrate,
            5.0 * 40.08 / 147.01 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert!(anhydrous > dihydrate);
        // A 33% w/w liquid: 15 g solution × 33% = 4.95 g anhydrous CaCl2.
        let liquid = ca(MineralForm::Liquid, 15.0, 33.0);
        assert_relative_eq!(
            liquid,
            4.95 * 40.08 / 110.98 / 20.0 * 1000.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn mineral_addition_chalk() {
        let res = calculate_water_treatment(
            WaterProfile::default(),
            10.0,
            &[MineralAddition {
                mineral_type: MineralType::Chalk,
                amount: 2.0,
                form: MineralForm::Hydrate,
                strength_pct: 0.0,
            }],
            None,
        )
        .unwrap();
        assert_relative_eq!(
            res.final_profile.calcium,
            2.0 * 40.08 / 100.09 / 10.0 * 1000.0,
            epsilon = 0.01
        );
        assert_relative_eq!(
            res.final_profile.bicarbonate,
            2.0 * 2.0 * 61.02 / 100.09 / 10.0 * 1000.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn mineral_addition_magnesium_chloride_hexahydrate() {
        // 10 g MgCl2·6H2O in 20 L → Mg ≈ 59.8, Cl ≈ 174.4 ppm.
        let res = calculate_water_treatment(
            WaterProfile::default(),
            20.0,
            &[MineralAddition {
                mineral_type: MineralType::MagnesiumCl,
                amount: 10.0,
                form: MineralForm::Hydrate,
                strength_pct: 0.0,
            }],
            None,
        )
        .unwrap();
        assert_relative_eq!(res.final_profile.magnesium, 59.8, epsilon = 0.2);
        assert_relative_eq!(res.final_profile.chloride, 174.4, epsilon = 0.2);
    }

    #[test]
    fn salt_forms_select_correct_molar_mass() {
        // Gypsum: anhydrous (136.14) is more concentrated than dihydrate (172.17).
        let ca = |form| {
            calculate_water_treatment(
                WaterProfile::default(),
                20.0,
                &[MineralAddition {
                    mineral_type: MineralType::Gypsum,
                    amount: 5.0,
                    form,
                    strength_pct: 0.0,
                }],
                None,
            )
            .unwrap()
            .final_profile
            .calcium
        };
        assert_relative_eq!(
            ca(MineralForm::Anhydrous),
            5.0 * 40.08 / 136.14 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert_relative_eq!(
            ca(MineralForm::Hydrate),
            5.0 * 40.08 / 172.17 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert!(ca(MineralForm::Anhydrous) > ca(MineralForm::Hydrate));

        // Na2SO4: anhydrous (142.04) is the default; the hydrate is Glauber's
        // decahydrate (322.20), giving less Na per gram.
        let na = |form| {
            calculate_water_treatment(
                WaterProfile::default(),
                20.0,
                &[MineralAddition {
                    mineral_type: MineralType::SodiumSulfate,
                    amount: 5.0,
                    form,
                    strength_pct: 0.0,
                }],
                None,
            )
            .unwrap()
            .final_profile
            .sodium
        };
        assert_relative_eq!(
            na(MineralForm::Anhydrous),
            5.0 * 2.0 * 22.99 / 142.04 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert_relative_eq!(
            na(MineralForm::Hydrate),
            5.0 * 2.0 * 22.99 / 322.20 / 20.0 * 1000.0,
            epsilon = 0.01
        );
        assert!(na(MineralForm::Anhydrous) > na(MineralForm::Hydrate));
    }

    #[test]
    fn calculate_acid_effect_phosphoric() {
        // 1 mL of 85% phosphoric in 10L
        let eff = calculate_acid_effect(
            AcidAddition {
                acid_type: AcidType::Phosphoric,
                strength: 85.0,
                amount: 1.0,
            },
            10.0,
        )
        .unwrap();
        assert!(eff.alk_reduction > 0.0);
        assert!(eff.phosphate > 0.0);
        assert_eq!(eff.sulfate, 0.0);
        assert_eq!(eff.chloride, 0.0);
        assert_relative_eq!(
            eff.alk_reduction,
            (eff.milli_equivs * 50.0) / 10.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn calculate_acid_effect_lactic() {
        let eff = calculate_acid_effect(
            AcidAddition {
                acid_type: AcidType::Lactic,
                strength: 88.0,
                amount: 1.0,
            },
            20.0,
        )
        .unwrap();
        assert!(eff.alk_reduction > 0.0);
        assert!(eff.lactate > 0.0);
        assert_eq!(eff.sulfate, 0.0);
    }

    #[test]
    fn calculate_acid_effect_zero_volume() {
        assert!(calculate_acid_effect(
            AcidAddition {
                acid_type: AcidType::Lactic,
                strength: 88.0,
                amount: 1.0,
            },
            0.0,
        )
        .is_err());
    }

    #[test]
    fn calculate_sparge_acidification_works() {
        // Moderate-alkalinity water: HCO3 = 150 ppm → alk ≈ 123 ppm CaCO3
        let profile = WaterProfile {
            bicarbonate: 150.0,
            ..Default::default()
        };
        let ml =
            calculate_sparge_acidification(profile, 15.0, 5.5, AcidType::Lactic, 88.0).unwrap();
        assert!(ml > 0.0);
    }

    #[test]
    fn convert_colour_to_lovibond_works() {
        assert_relative_eq!(
            convert_colour_to_lovibond(10.0, ColourUnit::Lovibond),
            10.0,
            epsilon = 0.001
        );
        assert_relative_eq!(
            convert_colour_to_lovibond(10.0, ColourUnit::Srm),
            10.0,
            epsilon = 0.001
        );
        // 26.5/2.65 = 10
        assert_relative_eq!(
            convert_colour_to_lovibond(26.5, ColourUnit::Ebc),
            10.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn blend_water_profiles_works() {
        let w1 = WaterProfile {
            calcium: 0.0,
            sodium: 100.0,
            ..Default::default()
        };
        let w2 = WaterProfile {
            calcium: 100.0,
            sodium: 0.0,
            ..Default::default()
        };

        let blended = blend_water_profiles(w1, w2, 0.5);
        assert_relative_eq!(blended.calcium, 50.0, epsilon = 0.01);
        assert_relative_eq!(blended.sodium, 50.0, epsilon = 0.01);

        let all1 = blend_water_profiles(w1, w2, 0.0);
        assert_relative_eq!(all1.calcium, 0.0, epsilon = 0.01);

        let all2 = blend_water_profiles(w1, w2, 1.0);
        assert_relative_eq!(all2.calcium, 100.0, epsilon = 0.01);
    }

    #[test]
    fn calculate_tds_works() {
        let p = WaterProfile {
            calcium: 100.0,
            magnesium: 10.0,
            sodium: 20.0,
            sulfate: 50.0,
            chloride: 30.0,
            bicarbonate: 80.0,
        };
        assert_relative_eq!(calculate_tds(p), 290.0, epsilon = 0.01);
    }

    #[test]
    fn calculate_hardness_works() {
        // Pure calcium: 100 ppm Ca → hardness = 100 × 50/20.04 ≈ 249.5 ppm CaCO3
        let p = WaterProfile {
            calcium: 100.0,
            ..Default::default()
        };
        assert_relative_eq!(calculate_hardness(p), 249.5, epsilon = 1.0);

        // Zero → zero
        assert_relative_eq!(
            calculate_hardness(WaterProfile::default()),
            0.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn format_water_profile_works() {
        let p = WaterProfile {
            calcium: 100.0,
            magnesium: 10.0,
            sodium: 20.0,
            sulfate: 50.0,
            chloride: 30.0,
            bicarbonate: 80.0,
        };
        let s = format_water_profile(p);
        assert!(s.contains("Ca:100"));
        assert!(s.contains("Mg:10"));
    }

    #[test]
    fn common_water_profiles_lookup() {
        for name in ["burton", "dublin", "pilsen", "ro"] {
            assert!(
                get_water_profile(name).is_some(),
                "profile {name:?} should exist"
            );
        }
        assert!(get_water_profile("nonexistent").is_none());

        // Case-insensitive lookup
        assert!(get_water_profile("Burton").is_some());
    }

    #[test]
    fn common_water_profiles_burton() {
        let p = get_water_profile("burton").unwrap();
        assert_relative_eq!(p.calcium, 295.0, epsilon = 1.0);
        assert_relative_eq!(p.sulfate, 725.0, epsilon = 1.0);
    }

    #[test]
    fn calculate_water_treatment_mash_ph() {
        // Pilsen-like water (low alkalinity) with pale malt should give mash pH ~5.5–6.0
        let profile = WaterProfile {
            bicarbonate: 15.0,
            ..Default::default()
        };
        let grains = vec![GrainAddition {
            name: "Pale".to_string(),
            grain_type: GrainType::Base,
            weight: 5.0,
            colour: 2.5,
        }];
        let res = calculate_water_treatment(
            profile,
            15.0,
            &[],
            Some(&MashParameters {
                water_volume: 15.0,
                grains,
                acids: vec![],
            }),
        )
        .unwrap();
        assert!(res.mash_ph > 4.0);
        assert!(res.mash_ph < 7.0);
    }

    #[test]
    fn calculate_water_treatment_no_mash() {
        let profile = WaterProfile {
            bicarbonate: 100.0,
            calcium: 50.0,
            ..Default::default()
        };
        let res = calculate_water_treatment(profile, 20.0, &[], None).unwrap();
        assert_eq!(res.mash_ph, 0.0);
        assert!(res.alkalinity > 0.0);
    }

    #[test]
    fn calculate_water_treatment_sulfate_chloride_ratio() {
        let profile = WaterProfile {
            sulfate: 100.0,
            chloride: 50.0,
            ..Default::default()
        };
        let res = calculate_water_treatment(profile, 20.0, &[], None).unwrap();
        assert_relative_eq!(res.sulfate_to_chloride, 2.0, epsilon = 0.01);
    }

    #[test]
    fn calculate_water_treatment_zero_volume() {
        assert!(calculate_water_treatment(WaterProfile::default(), 0.0, &[], None).is_err());
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn all_mineral_types() {
        let cases: Vec<(MineralType, fn(WaterProfile) -> f64)> = vec![
            (MineralType::Gypsum, |p| p.sulfate),
            (MineralType::CalciumCl, |p| p.calcium),
            (MineralType::Chalk, |p| p.bicarbonate),
            (MineralType::Epsom, |p| p.magnesium),
            (MineralType::MagnesiumCl, |p| p.chloride),
            (MineralType::BakingSoda, |p| p.bicarbonate),
            (MineralType::TableSalt, |p| p.chloride),
            (MineralType::SodiumSulfate, |p| p.sodium),
            (MineralType::SlakedLime, |p| p.calcium),
        ];
        for (mineral, ion_fn) in cases {
            let res = calculate_water_treatment(
                WaterProfile::default(),
                10.0,
                &[MineralAddition {
                    mineral_type: mineral,
                    amount: 5.0,
                    form: MineralForm::Hydrate,
                    strength_pct: 0.0,
                }],
                None,
            )
            .unwrap_or_else(|e| panic!("mineral={mineral:?}: {e}"));
            assert!(
                ion_fn(res.final_profile) > 0.0,
                "mineral={mineral:?} should increase ion"
            );
        }
    }

    #[test]
    fn all_acid_types() {
        for acid_type in [
            AcidType::Phosphoric,
            AcidType::Lactic,
            AcidType::Sulfuric,
            AcidType::Hydrochloric,
        ] {
            let eff = calculate_acid_effect(
                AcidAddition {
                    acid_type,
                    strength: 85.0,
                    amount: 1.0,
                },
                20.0,
            )
            .unwrap_or_else(|e| panic!("acid={acid_type:?}: {e}"));
            assert!(eff.alk_reduction > 0.0, "acid={acid_type:?}");
        }
        // unknown / unsupported acid should error
        assert!(calculate_acid_effect(
            AcidAddition {
                acid_type: AcidType::None,
                strength: 5.0,
                amount: 1.0,
            },
            20.0,
        )
        .is_err());
    }

    #[test]
    fn calculate_acid_effect_sulfuric_adds_sulfate() {
        let eff = calculate_acid_effect(
            AcidAddition {
                acid_type: AcidType::Sulfuric,
                strength: 98.0,
                amount: 1.0,
            },
            20.0,
        )
        .unwrap();
        assert!(eff.sulfate > 0.0);
    }

    #[test]
    fn calculate_acid_effect_hcl_adds_chloride() {
        let eff = calculate_acid_effect(
            AcidAddition {
                acid_type: AcidType::Hydrochloric,
                strength: 37.0,
                amount: 1.0,
            },
            20.0,
        )
        .unwrap();
        assert!(eff.chloride > 0.0);
    }

    #[test]
    fn all_grain_types_mash_ph() {
        // Exercise all grain type branches in grain_contrib.
        let profile = WaterProfile {
            bicarbonate: 100.0,
            ..Default::default()
        };
        let grains = vec![
            GrainAddition {
                name: "Pale Malt".to_string(),
                grain_type: GrainType::Base,
                weight: 4.0,
                colour: 2.5,
            },
            GrainAddition {
                name: "Crystal 60".to_string(),
                grain_type: GrainType::Crystal,
                weight: 0.5,
                colour: 60.0,
            },
            GrainAddition {
                name: "Chocolate".to_string(),
                grain_type: GrainType::Roast,
                weight: 0.3,
                colour: 340.0,
            },
            GrainAddition {
                name: "Acid Malt".to_string(),
                grain_type: GrainType::Acid,
                weight: 0.2,
                colour: 3.0,
            },
        ];
        let res = calculate_water_treatment(
            profile,
            15.0,
            &[],
            Some(&MashParameters {
                water_volume: 15.0,
                grains,
                acids: vec![],
            }),
        )
        .unwrap();
        assert!(res.mash_ph > 4.0);
        assert!(res.mash_ph < 7.0);
    }

    #[test]
    fn calculate_water_treatment_with_acid_in_mash() {
        // Acid addition in MashParameters should reduce alkalinity.
        let profile = WaterProfile {
            bicarbonate: 200.0,
            ..Default::default()
        };
        let res = calculate_water_treatment(
            profile,
            20.0,
            &[],
            Some(&MashParameters {
                water_volume: 20.0,
                grains: vec![GrainAddition {
                    name: "Pale".to_string(),
                    grain_type: GrainType::Base,
                    weight: 5.0,
                    colour: 2.5,
                }],
                acids: vec![AcidAddition {
                    acid_type: AcidType::Lactic,
                    strength: 88.0,
                    amount: 2.0,
                }],
            }),
        )
        .unwrap();
        // Final bicarbonate should be less than the starting 200 ppm.
        assert!(res.final_profile.bicarbonate < 200.0);
    }

    #[test]
    fn calculate_sparge_acidification_zero_volume() {
        assert!(calculate_sparge_acidification(
            WaterProfile {
                bicarbonate: 100.0,
                ..Default::default()
            },
            0.0,
            5.5,
            AcidType::Lactic,
            88.0,
        )
        .is_err());
    }

    #[test]
    fn calculate_sparge_acidification_unknown_acid() {
        assert!(calculate_sparge_acidification(
            WaterProfile {
                bicarbonate: 100.0,
                ..Default::default()
            },
            10.0,
            5.5,
            AcidType::None,
            5.0,
        )
        .is_err());
    }
}
