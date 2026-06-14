//! Beer colour calculations using the Morey equation.
//!
//! Port of the Go `pkg/color` package.

const KG_TO_LBS: f64 = 2.20462262;
const LITRES_TO_US_GAL: f64 = 0.26417205;

/// A grain addition for colour calculation.
#[derive(Debug, Clone, Copy)]
pub struct FermentableEntry {
    pub amount_kg: f64,
    pub color_ebc: f64,
}

/// Returns the predicted beer colour in SRM using the Morey equation.
///
/// Grain colour is accepted in EBC; batch volume in litres.
pub fn calculate_srm(grains: &[FermentableEntry], batch_vol_l: f64) -> f64 {
    if grains.is_empty() || batch_vol_l <= 0.0 {
        return 0.0;
    }
    let vol_gal = batch_vol_l * LITRES_TO_US_GAL;
    let mut mcu = 0.0;
    for g in grains {
        let lovibond = g.color_ebc / 2.65;
        let weight_lbs = g.amount_kg * KG_TO_LBS;
        mcu += (lovibond * weight_lbs) / vol_gal;
    }
    if mcu <= 0.0 {
        return 0.0;
    }
    1.4922 * mcu.powf(0.6859)
}

/// Converts SRM to EBC (EBC = SRM × 1.97).
pub fn srm_to_ebc(srm: f64) -> f64 {
    srm * 1.97
}

/// Converts EBC to SRM (SRM = EBC / 1.97).
pub fn ebc_to_srm(ebc: f64) -> f64 {
    ebc / 1.97
}

/// Converts degrees Lovibond to EBC using the industry formula.
pub fn lovibond_to_ebc(lovibond: f64) -> f64 {
    (lovibond * 2.65 - 1.2) * 1.97
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn empty_grain_bill_is_zero() {
        assert_eq!(calculate_srm(&[], 20.0), 0.0);
        assert_eq!(
            calculate_srm(
                &[FermentableEntry {
                    amount_kg: 5.0,
                    color_ebc: 7.0
                }],
                0.0
            ),
            0.0
        );
    }

    #[test]
    fn pale_ale_in_expected_range() {
        let grains = [FermentableEntry {
            amount_kg: 5.0,
            color_ebc: 7.0,
        }];
        let srm = calculate_srm(&grains, 20.0);
        assert!(srm > 2.0 && srm < 8.0, "srm was {srm}");
    }

    #[test]
    fn ebc_srm_roundtrip() {
        assert_relative_eq!(ebc_to_srm(srm_to_ebc(10.0)), 10.0, epsilon = 1e-9);
    }

    #[test]
    fn lovibond_conversion() {
        assert_relative_eq!(
            lovibond_to_ebc(2.0),
            (2.0 * 2.65 - 1.2) * 1.97,
            epsilon = 1e-9
        );
    }
}
