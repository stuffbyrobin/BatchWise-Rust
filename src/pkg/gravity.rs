//! Brewing gravity calculations.
//!
//! Port of the Go `pkg/gravity` package. Fallible operations return
//! [`Result<f64, GravityError>`] in place of Go's `(float64, error)` tuples.

use std::fmt;

// Unit constants from §1 of PHYSICS_REFERENCE.md.
pub const LBS_TO_KG: f64 = 0.45359237;
pub const KG_TO_LBS: f64 = 2.20462262;
pub const US_GAL_TO_LITRES: f64 = 3.78541178;
pub const IMP_GAL_TO_LITRES: f64 = 4.54609;
pub const LITRES_TO_US_GAL: f64 = 0.26417205;
pub const LITRES_TO_IMP_GAL: f64 = 0.21996925;
pub const PPG_TO_LDK_FACTOR: f64 = 8.345404;
pub const LDK_TO_PPG_FACTOR: f64 = 0.11988;

pub const SUCROSE_LDK_IOB: f64 = 386.0;
pub const SUCROSE_LDK_ASBC: f64 = 46.214 * PPG_TO_LDK_FACTOR; // ≈ 385.6648

/// Errors returned by the gravity calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum GravityError {
    /// A numeric input failed a precondition (the message describes which).
    Invalid(String),
}

impl fmt::Display for GravityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GravityError::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for GravityError {}

/// Converts specific gravity to degrees Plato using the ASBC polynomial.
pub fn sg_to_plato(sg: f64) -> f64 {
    -616.868 + 1111.14 * sg - 630.272 * sg * sg + 135.997 * sg * sg * sg
}

/// Converts degrees Plato to specific gravity via Newton-Raphson iteration.
pub fn plato_to_sg(plato: f64) -> f64 {
    let mut sg = 1.0 + plato / 250.0;
    for _ in 0..10 {
        let f = sg_to_plato(sg) - plato;
        let fp = 1111.14 - 2.0 * 630.272 * sg + 3.0 * 135.997 * sg * sg;
        sg -= f / fp;
    }
    sg
}

/// Returns ABV% using the standard formula (OG−FG)×131.25.
pub fn calculate_abv(og: f64, fg: f64) -> Result<f64, GravityError> {
    validate_og_fg(og, fg)?;
    Ok((og - fg) * 131.25)
}

/// Returns ABV% using the high-gravity formula (more accurate above ~8% ABV).
pub fn calculate_abv_alternative(og: f64, fg: f64) -> Result<f64, GravityError> {
    validate_og_fg(og, fg)?;
    Ok(76.08 * (og - fg) / (1.775 - og) * (fg / 0.794))
}

/// Returns apparent attenuation % as ((OG−FG)/(OG−1))×100.
pub fn calculate_attenuation(og: f64, fg: f64) -> Result<f64, GravityError> {
    if og <= 1.0 {
        return Err(GravityError::Invalid(format!(
            "OG must be > 1.0, got {og:.4}"
        )));
    }
    Ok(((og - fg) / (og - 1.0)) * 100.0)
}

/// Returns the predicted FG from OG and apparent attenuation %.
pub fn estimate_final_gravity(og: f64, attenuation_percent: f64) -> Result<f64, GravityError> {
    if og <= 1.0 {
        return Err(GravityError::Invalid(format!(
            "OG must be > 1.0, got {og:.4}"
        )));
    }
    Ok(1.0 + (og - 1.0) * (1.0 - attenuation_percent / 100.0))
}

/// Returns the new SG after diluting `original_vol` litres of wort at `og` with
/// `dilution_vol` litres of water.
pub fn dilution_gravity(
    og: f64,
    original_vol: f64,
    dilution_vol: f64,
) -> Result<f64, GravityError> {
    if og <= 1.0 {
        return Err(GravityError::Invalid(format!(
            "OG must be > 1.0, got {og:.4}"
        )));
    }
    if original_vol <= 0.0 {
        return Err(GravityError::Invalid(format!(
            "originalVol must be > 0, got {original_vol:.4}"
        )));
    }
    let new_vol = original_vol + dilution_vol;
    Ok(1.0 + (og - 1.0) * original_vol / new_vol)
}

/// Returns the new SG after boiling down `original_vol` litres to `final_vol` litres.
pub fn concentration_gravity(
    og: f64,
    original_vol: f64,
    final_vol: f64,
) -> Result<f64, GravityError> {
    if og <= 1.0 {
        return Err(GravityError::Invalid(format!(
            "OG must be > 1.0, got {og:.4}"
        )));
    }
    if final_vol > original_vol {
        return Err(GravityError::Invalid(format!(
            "finalVol ({final_vol:.4}) must be <= originalVol ({original_vol:.4})"
        )));
    }
    if final_vol <= 0.0 {
        return Err(GravityError::Invalid(format!(
            "finalVol must be > 0, got {final_vol:.4}"
        )));
    }
    Ok(1.0 + (og - 1.0) * original_vol / final_vol)
}

/// Returns the boil-off rate in L/hr.
pub fn boil_off_rate(pre_l: f64, post_l: f64, hours: f64) -> Result<f64, GravityError> {
    if hours <= 0.0 {
        return Err(GravityError::Invalid(format!(
            "hours must be > 0, got {hours:.4}"
        )));
    }
    Ok((pre_l - post_l) / hours)
}

/// Returns the expected post-boil volume.
pub fn estimate_post_boil_volume(
    pre_l: f64,
    rate_per_hr: f64,
    hours: f64,
) -> Result<f64, GravityError> {
    if hours < 0.0 {
        return Err(GravityError::Invalid(format!(
            "hours must be >= 0, got {hours:.4}"
        )));
    }
    Ok(pre_l - rate_per_hr * hours)
}

/// Returns kcal per 100 mL using the Brewers Publications formula.
pub fn calculate_calories(og: f64, fg: f64) -> Result<f64, GravityError> {
    let abv = calculate_abv(og, fg)?;
    let og_pts = (og - 1.0) * 1000.0;
    let fg_pts = (fg - 1.0) * 1000.0;
    let real_extract = 0.1808 * og_pts + 0.8192 * fg_pts;
    Ok(6.9 * abv + 4.0 * real_extract / 1000.0 * 100.0)
}

/// Returns the arithmetic mean of the provided SG values; empty returns 1.0.
pub fn combine_gravities(sgs: &[f64]) -> f64 {
    if sgs.is_empty() {
        return 1.0;
    }
    sgs.iter().sum::<f64>() / sgs.len() as f64
}

/// Returns brew-house efficiency as (actualSG−1)/(expectedSG−1)×100.
pub fn calculate_efficiency(expected_sg: f64, actual_sg: f64) -> Result<f64, GravityError> {
    if expected_sg <= 1.0 {
        return Err(GravityError::Invalid(format!(
            "expectedSG must be > 1.0, got {expected_sg:.4}"
        )));
    }
    Ok((actual_sg - 1.0) / (expected_sg - 1.0) * 100.0)
}

/// Returns true when SG is in the range [0.990, 1.200].
pub fn is_valid_sg(sg: f64) -> bool {
    (0.990..=1.200).contains(&sg)
}

/// Returns true when Plato is in the range [−5, 50].
pub fn is_valid_plato(p: f64) -> bool {
    (-5.0..=50.0).contains(&p)
}

/// Sucrose reference for yield-based potential conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrewingStandard {
    /// UK standard (default).
    Iob,
    /// US imports.
    Asbc,
    /// Uses the same reference as IOB.
    Ebc,
}

/// Malt weight input unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassUnit {
    Kg,
    Lbs,
}

/// Batch volume input unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeUnit {
    Litres,
    UsGal,
    ImpGal,
}

/// How malt extract potential is expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotentialUnit {
    /// Litre-degrees per kg.
    Ldk,
    /// Points per pound per gallon.
    Ppg,
    /// Percent of sucrose reference.
    Yield,
    /// Specific gravity, e.g. 1.037.
    Sg,
}

/// A single malt/grain addition for OG calculation.
#[derive(Debug, Clone, Copy)]
pub struct MaltAddition {
    pub mass_amount: f64,
    pub mass_unit: MassUnit,
    pub potential_value: f64,
    pub potential_unit: PotentialUnit,
    pub volume_amount: f64,
    pub volume_unit: VolumeUnit,
    /// Brew-house efficiency, 0–100.
    pub efficiency: f64,
}

/// Output of [`Calculator::calculate`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalcResult {
    pub specific_gravity: f64,
    pub plato: f64,
    /// Gravity points = (SG−1)×1000.
    pub metric_points: f64,
}

/// Computes wort OG from grain-bill parameters.
#[derive(Debug, Clone, Copy)]
pub struct Calculator {
    standard: BrewingStandard,
}

impl Calculator {
    /// Creates a calculator using the given brewing standard.
    pub fn new(standard: BrewingStandard) -> Self {
        Self { standard }
    }

    /// Returns the predicted OG for a single malt addition.
    pub fn calculate(&self, input: MaltAddition) -> Result<CalcResult, GravityError> {
        let mass_kg = to_kg(input.mass_amount, input.mass_unit);
        let vol_l = to_litres(input.volume_amount, input.volume_unit);
        if vol_l <= 0.0 {
            return Err(GravityError::Invalid("volume must be > 0".to_string()));
        }
        let ldk = self.potential_to_ldk(input.potential_value, input.potential_unit);
        let extract = mass_kg * ldk * (input.efficiency / 100.0);
        let gp = extract / vol_l;
        let sg = 1.0 + gp / 1000.0;
        Ok(CalcResult {
            specific_gravity: sg,
            plato: sg_to_plato(sg),
            metric_points: gp,
        })
    }

    fn potential_to_ldk(&self, value: f64, unit: PotentialUnit) -> f64 {
        match unit {
            PotentialUnit::Ldk => value,
            PotentialUnit::Ppg => value * PPG_TO_LDK_FACTOR,
            PotentialUnit::Yield => (value / 100.0) * self.sucrose_ldk(),
            PotentialUnit::Sg => (value - 1.0) * 1000.0 * PPG_TO_LDK_FACTOR,
        }
    }

    fn sucrose_ldk(&self) -> f64 {
        match self.standard {
            BrewingStandard::Asbc => SUCROSE_LDK_ASBC,
            BrewingStandard::Iob | BrewingStandard::Ebc => SUCROSE_LDK_IOB,
        }
    }
}

fn to_kg(amount: f64, unit: MassUnit) -> f64 {
    match unit {
        MassUnit::Lbs => amount * LBS_TO_KG,
        MassUnit::Kg => amount,
    }
}

fn to_litres(amount: f64, unit: VolumeUnit) -> f64 {
    match unit {
        VolumeUnit::UsGal => amount * US_GAL_TO_LITRES,
        VolumeUnit::ImpGal => amount * IMP_GAL_TO_LITRES,
        VolumeUnit::Litres => amount,
    }
}

fn validate_og_fg(og: f64, fg: f64) -> Result<(), GravityError> {
    if og <= 1.0 {
        return Err(GravityError::Invalid(format!(
            "OG must be > 1.0, got {og:.4}"
        )));
    }
    if fg <= 0.0 {
        return Err(GravityError::Invalid(format!(
            "FG must be > 0, got {fg:.4}"
        )));
    }
    if fg > og {
        return Err(GravityError::Invalid(format!(
            "FG ({fg:.4}) must be <= OG ({og:.4})"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn sg_plato_roundtrip() {
        let sg = 1.048;
        let plato = sg_to_plato(sg);
        assert_relative_eq!(plato_to_sg(plato), sg, epsilon = 1e-6);
    }

    #[test]
    fn abv_standard() {
        assert_relative_eq!(calculate_abv(1.050, 1.010).unwrap(), 5.25, epsilon = 1e-9);
    }

    #[test]
    fn abv_rejects_bad_input() {
        assert!(calculate_abv(1.0, 1.010).is_err());
        assert!(calculate_abv(1.050, 1.060).is_err());
        assert!(calculate_abv(1.050, 0.0).is_err());
    }

    #[test]
    fn attenuation() {
        assert_relative_eq!(
            calculate_attenuation(1.050, 1.010).unwrap(),
            80.0,
            epsilon = 1e-9
        );
    }

    #[test]
    fn dilution_lowers_gravity() {
        let sg = dilution_gravity(1.060, 20.0, 5.0).unwrap();
        assert!(sg < 1.060 && sg > 1.0);
    }

    #[test]
    fn concentration_raises_gravity() {
        let sg = concentration_gravity(1.040, 25.0, 20.0).unwrap();
        assert!(sg > 1.040);
    }

    #[test]
    fn combine_empty_is_water() {
        assert_eq!(combine_gravities(&[]), 1.0);
    }

    #[test]
    fn validity_ranges() {
        assert!(is_valid_sg(1.050));
        assert!(!is_valid_sg(1.300));
        assert!(is_valid_plato(12.0));
        assert!(!is_valid_plato(60.0));
    }

    #[test]
    fn calculator_ppg() {
        let calc = Calculator::new(BrewingStandard::Iob);
        let res = calc
            .calculate(MaltAddition {
                mass_amount: 5.0,
                mass_unit: MassUnit::Kg,
                potential_value: 37.0,
                potential_unit: PotentialUnit::Ppg,
                volume_amount: 20.0,
                volume_unit: VolumeUnit::Litres,
                efficiency: 75.0,
            })
            .unwrap();
        assert!(res.specific_gravity > 1.0);
        assert_relative_eq!(
            res.metric_points,
            (res.specific_gravity - 1.0) * 1000.0,
            epsilon = 1e-9
        );
    }
}
