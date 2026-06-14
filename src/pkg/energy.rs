//! Brewing energy and cost calculations.
//!
//! Port of the Go `pkg/energy` package.

/// kJ/(kg·°C); 1 L water ≈ 1 kg.
const SPECIFIC_HEAT_WATER: f64 = 4.186;

/// Returns the energy in kJ required to heat (or cool) `volume_l` litres of
/// water from `start_temp_c` to `end_temp_c`.
pub fn calculate_heating(volume_l: f64, start_temp_c: f64, end_temp_c: f64) -> f64 {
    SPECIFIC_HEAT_WATER * volume_l * (end_temp_c - start_temp_c)
}

/// Converts kilojoules into pence using the supplied rate.
///
/// Uses banker's rounding (round-half-to-even) for consistency with the duty
/// calculator.
pub fn calculate_energy_cost(kilojoules: f64, rate_pence_per_kwh: i64) -> i64 {
    let kwh = kilojoules / 3600.0;
    let pence = kwh * rate_pence_per_kwh as f64;
    round_half_to_even(pence) as i64
}

/// Round-half-to-even, matching Go's `math.RoundToEven`.
fn round_half_to_even(x: f64) -> f64 {
    let rounded = x.round();
    if (x - x.trunc()).abs() == 0.5 {
        // Exactly halfway: pick the even neighbour.
        let floor = x.floor();
        if (floor as i64) % 2 == 0 {
            floor
        } else {
            floor + 1.0
        }
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn heating_positive() {
        // 20 L from 20°C to 100°C.
        let kj = calculate_heating(20.0, 20.0, 100.0);
        assert_relative_eq!(kj, 4.186 * 20.0 * 80.0, epsilon = 1e-9);
    }

    #[test]
    fn cooling_negative() {
        assert!(calculate_heating(20.0, 100.0, 20.0) < 0.0);
    }

    #[test]
    fn energy_cost_rounds_half_even() {
        // 3600 kJ = 1 kWh; at 30p/kWh = 30p.
        assert_eq!(calculate_energy_cost(3600.0, 30), 30);
    }
}
