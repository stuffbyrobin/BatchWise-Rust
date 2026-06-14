//! UK HMRC Beer Duty calculations.
//!
//! Port of the Go `pkg/duty` package. Duty estimates are used both for batch
//! cost planning and for actual crystallisation on sale. Monetary values are
//! kept in `i64` pence, matching the Go source. The Go `(T, error)` return
//! shapes become [`Result<T, DutyError>`]; functions that are infallible in Go
//! stay infallible here.

use std::fmt;

/// Errors returned by the duty calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum DutyError {
    /// An input failed a precondition (the message describes which).
    Invalid(String),
}

impl fmt::Display for DutyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DutyError::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DutyError {}

/// Rounds `x` to the nearest integer using banker's rounding (round half to
/// even), returning the result as `i64`.
///
/// Replicates Go's `math.RoundToEven` semantics for the values produced by the
/// duty calculations. Like Go, integer truncation/modulo are toward zero, so
/// the even/odd tie-break matches for negative inputs as well.
fn round_to_even(x: f64) -> i64 {
    let floor = x.floor();
    let diff = x - floor;
    if diff < 0.5 {
        floor as i64
    } else if diff > 0.5 {
        floor as i64 + 1
    } else if (floor as i64) % 2 == 0 {
        floor as i64
    } else {
        floor as i64 + 1
    }
}

/// Implements the UK HMRC 2024 Beer Duty brackets.
///
/// Bracket boundaries are inclusive on the lower bound, exclusive on the upper.
///
/// ```text
/// [0, 1.2)   → £0.00 per hL per %ABV
/// [1.2, 2.8) → £8.42  per hL per %ABV
/// [2.8, 7.5) → £19.08 per hL per %ABV
/// [7.5, ∞)   → £24.77 per hL per %ABV  (£19.08 + £5.69)
/// ```
///
/// Returns the duty in pence.
pub fn calculate_beer_duty_gb(volume_liters: f64, abv_pct: f64) -> i64 {
    let rate_per_hl_per_pct = if abv_pct < 1.2 {
        return 0;
    } else if abv_pct < 2.8 {
        8.42
    } else if abv_pct < 7.5 {
        19.08
    } else {
        24.77
    };

    let hl = volume_liters / 100.0;
    let duty_pounds = hl * abv_pct * rate_per_hl_per_pct;
    round_to_even(duty_pounds * 100.0)
}

/// Returns the HMRC Small Producer Relief fraction in `[0.0, 0.5]` for the given
/// annual production in hL pure alcohol (hLPA).
///
/// Products ≥ 8.5% ABV are ineligible — callers must pass 0 for those.
///
/// ```text
/// ≤ 0 or > 4500 hLPA → 0 (ineligible)
/// ≤ 2100 hLPA        → 0.5 (maximum 50 % relief)
/// 2100 < x ≤ 4500    → (4500 − x) / 4800 (sliding scale)
/// ```
pub fn spr_relief_rate(annual_production_hl_pa: f64) -> f64 {
    if annual_production_hl_pa <= 0.0 {
        0.0
    } else if annual_production_hl_pa <= 2100.0 {
        0.5
    } else if annual_production_hl_pa <= 4500.0 {
        (4500.0 - annual_production_hl_pa) / 4800.0
    } else {
        0.0
    }
}

/// Returns the duty estimate in pence for the given jurisdiction, volume
/// (litres), and ABV (percent).
///
/// Unknown jurisdictions return `Err(DutyError::Invalid)`. The Go original logs
/// a WARN and returns 0 (failing open) using an injected `*slog.Logger`; this
/// port has no logger dependency, so the unsupported case surfaces as an error
/// that the caller can decide to treat as zero.
pub fn calculate_duty(
    jurisdiction: &str,
    volume_liters: f64,
    abv_pct: f64,
) -> Result<i64, DutyError> {
    match jurisdiction.to_uppercase().as_str() {
        "GB" => Ok(calculate_beer_duty_gb(volume_liters, abv_pct)),
        other => Err(DutyError::Invalid(format!(
            "duty calculation requested for unsupported jurisdiction {other}; returning 0"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn calculate_beer_duty_gb_brackets() {
        let cases: &[(&str, f64, f64, i64)] = &[
            ("below 1.2% — no duty", 100.0, 0.8, 0),
            ("exactly 1.2% — lowest bracket", 100.0, 1.2, 1010),
            ("exactly 2.8% — mid bracket", 100.0, 2.8, 5342),
            ("5.5% — mid bracket", 100.0, 5.5, 10494),
            ("exactly 7.5% — high bracket", 100.0, 7.5, 18578),
            ("1000L at 4.5%", 1000.0, 4.5, 85860),
        ];
        for (name, volume, abv, want) in cases {
            let got = calculate_beer_duty_gb(*volume, *abv);
            assert_eq!(got, *want, "{name}");
        }
    }

    #[test]
    fn calculate_duty_gb_delegates_correctly() {
        for abv in [0.8, 1.2, 2.8, 5.5, 7.5] {
            assert_eq!(
                calculate_duty("GB", 100.0, abv).unwrap(),
                calculate_beer_duty_gb(100.0, abv),
                "GB dispatch mismatch for abv={abv:.1}",
            );
        }
    }

    #[test]
    fn calculate_duty_lowercase_gb_case_insensitive() {
        assert_eq!(
            calculate_duty("gb", 100.0, 5.0).unwrap(),
            calculate_beer_duty_gb(100.0, 5.0),
        );
    }

    #[test]
    fn calculate_duty_us_returns_error() {
        assert!(calculate_duty("US", 100.0, 5.0).is_err());
    }

    #[test]
    fn calculate_duty_unknown_jurisdiction_returns_error() {
        assert!(calculate_duty("AU", 50.0, 4.5).is_err());
    }

    #[test]
    fn spr_relief_rate_brackets() {
        let cases: &[(&str, f64, f64)] = &[
            ("zero production — ineligible", 0.0, 0.0),
            ("negative — ineligible", -1.0, 0.0),
            ("small producer max relief", 2100.0, 0.5),
            ("below max threshold", 1000.0, 0.5),
            (
                "just above max relief boundary",
                2101.0,
                (4500.0 - 2101.0) / 4800.0,
            ),
            ("mid sliding scale", 3300.0, (4500.0 - 3300.0) / 4800.0),
            ("just below cutoff", 4499.0, (4500.0 - 4499.0) / 4800.0),
            ("exactly at cutoff", 4500.0, 0.0),
            ("above cutoff — ineligible", 4501.0, 0.0),
            ("large producer", 100000.0, 0.0),
        ];
        for (_name, production, want) in cases {
            let got = spr_relief_rate(*production);
            assert_relative_eq!(got, *want, epsilon = 1e-9);
        }
    }

    #[test]
    fn round_to_even_half_even_cases() {
        assert_eq!(round_to_even(2.5), 2);
        assert_eq!(round_to_even(3.5), 4);
        assert_eq!(round_to_even(1.5), 2);
        assert_eq!(round_to_even(-1.5), -2);
    }
}
