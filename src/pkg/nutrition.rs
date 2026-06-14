//! Beer nutrition / energy-label calculations.
//!
//! Port of the Go `pkg/nutrition` package. These are pure functions for
//! estimating beer energy content and UK alcohol units. None of the source
//! functions are fallible, so none return `Result`; the [`NutritionError`]
//! enum is provided for parity with the wider Go→Rust conversion.

use std::fmt;

const ETHANOL_DENSITY: f64 = 0.789; // g/mL
const ETHANOL_ENERGY_KJ: f64 = 29.0; // kJ/g (HMRC factor)
const KJ_PER_KCAL: f64 = 4.184;
const GRAMS_PER_UNIT: f64 = 8.0; // 1 UK alcohol unit = 8 g pure ethanol

/// Errors returned by the nutrition calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum NutritionError {
    /// A numeric input failed a precondition (the message describes which).
    Invalid(String),
}

impl fmt::Display for NutritionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NutritionError::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for NutritionError {}

/// Returns estimated energy in kJ per 100 mL of beer.
///
/// Formula: (abv_pct / 100) × 100 mL × 0.789 g/mL × 29 kJ/g
pub fn energy_kj_per_100ml(abv_pct: f64) -> f64 {
    (abv_pct / 100.0) * 100.0 * ETHANOL_DENSITY * ETHANOL_ENERGY_KJ
}

/// Returns estimated energy in kcal per 100 mL of beer.
pub fn energy_kcal_per_100ml(abv_pct: f64) -> f64 {
    energy_kj_per_100ml(abv_pct) / KJ_PER_KCAL
}

/// Returns UK alcohol units for a single container.
///
/// Formula: (abv_pct / 100) × volume_ml × 0.789 / 8
pub fn alcohol_units(abv_pct: f64, volume_ml: f64) -> f64 {
    (abv_pct / 100.0) * volume_ml * ETHANOL_DENSITY / GRAMS_PER_UNIT
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_energy_kj_per_100ml() {
        // 5% ABV: (5/100) * 100 * 0.789 * 29 = 114.405 kJ
        assert_relative_eq!(energy_kj_per_100ml(5.0), 114.405, epsilon = 0.01);
        assert_eq!(energy_kj_per_100ml(0.0), 0.0);
    }

    #[test]
    fn test_energy_kcal_per_100ml() {
        // 114.405 / 4.184 ≈ 27.34 kcal
        assert_relative_eq!(energy_kcal_per_100ml(5.0), 27.34, epsilon = 0.01);
        assert_eq!(energy_kcal_per_100ml(0.0), 0.0);
    }

    #[test]
    fn test_alcohol_units() {
        // 500mL at 5% ABV: 500 * 0.05 * 0.789 / 8 = 2.466 units
        assert_relative_eq!(alcohol_units(5.0, 500.0), 2.466, epsilon = 0.01);
        assert_eq!(alcohol_units(0.0, 500.0), 0.0);
        assert_eq!(alcohol_units(5.0, 0.0), 0.0);
    }
}
