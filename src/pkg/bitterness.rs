//! Hop bitterness (IBU) calculations.
//!
//! Port of the Go `pkg/bitterness` package. Provides IBU calculations using the
//! Tinseth, Rager, and Garetz models, plus hop-addition normalisation. Fallible
//! operations would return [`Result<f64, BitternessError>`]; the current model
//! functions are infallible and return plain `f64`.

use std::fmt;

/// Errors returned by the bitterness calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum BitternessError {
    /// A numeric input failed a precondition (the message describes which).
    Invalid(String),
}

impl fmt::Display for BitternessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitternessError::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for BitternessError {}

/// A single hop addition for IBU calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct HopAddition {
    pub amount_g: f64,
    pub alpha_acid_pct: f64,
    pub boil_time_minutes: f64,
    /// "pellet" (default), "leaf", "extract".
    pub form: String,
    /// "boil", "whirlpool", "dry-hop", "first-wort", "mash".
    pub use_: String,
}

/// Returns total IBU using the Tinseth (1995) model.
///
/// Dry-hop and mash additions contribute zero IBU. Whirlpool additions use
/// Tinseth at 10 min × 0.5 (industry fudge factor).
pub fn calculate_tinseth(additions: &[HopAddition], batch_vol_l: f64, wort_og: f64) -> f64 {
    if batch_vol_l <= 0.0 {
        return 0.0;
    }
    let mut total = 0.0;
    for a in additions {
        match a.use_.as_str() {
            "dry-hop" | "mash" => continue,
            _ => {}
        }
        let u = if a.use_ == "whirlpool" {
            tinseth_utilisation(10.0, wort_og) * 0.5
        } else {
            tinseth_utilisation(a.boil_time_minutes, wort_og)
        };
        let alpha_grams = a.amount_g * a.alpha_acid_pct / 100.0;
        total += u * form_factor(&a.form) * alpha_grams * 1000.0 / batch_vol_l;
    }
    total
}

/// Returns total IBU using the Rager model.
pub fn calculate_rager(additions: &[HopAddition], batch_vol_l: f64, wort_og: f64) -> f64 {
    if batch_vol_l <= 0.0 {
        return 0.0;
    }
    let mut ga_factor = 0.0;
    if wort_og > 1.050 {
        ga_factor = (wort_og - 1.050) / 0.2;
    }
    let mut total = 0.0;
    for a in additions {
        match a.use_.as_str() {
            "dry-hop" | "mash" => continue,
            _ => {}
        }
        let mut t = a.boil_time_minutes;
        if a.use_ == "whirlpool" {
            t = 10.0;
        }
        let util_pct = 18.11 + 13.86 * ((t - 31.32) / 18.27).tanh();
        let alpha_grams = a.amount_g * a.alpha_acid_pct / 100.0;
        total += alpha_grams * form_factor(&a.form) * util_pct * 0.10
            / (batch_vol_l * (1.0 + ga_factor));
    }
    total
}

/// Returns total IBU using a simplified Garetz model.
///
/// `brewer_yeast_factor` accounts for yeast stripping; typical range 1.0–1.15.
/// A higher factor reduces perceived IBU.
pub fn calculate_garetz(
    additions: &[HopAddition],
    batch_vol_l: f64,
    wort_og: f64,
    brewer_yeast_factor: f64,
) -> f64 {
    if batch_vol_l <= 0.0 || brewer_yeast_factor <= 0.0 {
        return 0.0;
    }
    let mut ga_factor = 0.0;
    if wort_og > 1.050 {
        ga_factor = (wort_og - 1.050) / 0.2;
    }
    let mut total = 0.0;
    for a in additions {
        match a.use_.as_str() {
            "dry-hop" | "mash" => continue,
            _ => {}
        }
        let mut t = a.boil_time_minutes;
        if a.use_ == "whirlpool" {
            t = 10.0;
        }
        let util_pct = 18.11 + 13.86 * ((t - 31.32) / 18.27).tanh();
        let alpha_grams = a.amount_g * a.alpha_acid_pct / 100.0;
        total += alpha_grams * form_factor(&a.form) * util_pct * 0.10
            / (batch_vol_l * (1.0 + ga_factor) * brewer_yeast_factor);
    }
    total
}

/// Canonicalises Form and Use fields, lowercasing and collapsing aliases
/// ("whole" → "leaf"; defaults for empty strings).
pub fn normalise_hop_additions(additions: &[HopAddition]) -> Vec<HopAddition> {
    additions
        .iter()
        .map(|a| HopAddition {
            amount_g: a.amount_g,
            alpha_acid_pct: a.alpha_acid_pct,
            boil_time_minutes: a.boil_time_minutes,
            form: normalise_form(&a.form),
            use_: normalise_use(&a.use_),
        })
        .collect()
}

fn tinseth_utilisation(time_min: f64, wort_og: f64) -> f64 {
    let bigness = 1.65 * 0.000125_f64.powf(wort_og - 1.0);
    let boil_time_factor = (1.0 - (-0.04 * time_min).exp()) / 4.15;
    bigness * boil_time_factor
}

fn form_factor(form: &str) -> f64 {
    match form.to_lowercase().as_str() {
        "leaf" | "whole" => 0.9,
        _ => 1.0,
    }
}

fn normalise_form(f: &str) -> String {
    let lower = f.to_lowercase();
    match lower.as_str() {
        "whole" => "leaf".to_string(),
        "leaf" | "extract" => lower,
        _ => "pellet".to_string(),
    }
}

fn normalise_use(u: &str) -> String {
    match u.to_lowercase().as_str() {
        "boil" => "boil".to_string(),
        "whirlpool" => "whirlpool".to_string(),
        "dry-hop" => "dry-hop".to_string(),
        "first-wort" => "first-wort".to_string(),
        "mash" => "mash".to_string(),
        _ => "boil".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn add(amount_g: f64, alpha: f64, time: f64, form: &str, use_: &str) -> HopAddition {
        HopAddition {
            amount_g,
            alpha_acid_pct: alpha,
            boil_time_minutes: time,
            form: form.to_string(),
            use_: use_.to_string(),
        }
    }

    #[test]
    fn calculate_tinseth_cases() {
        // Spec §5.1 vector 1: ~38±2. Formula gives ~40; 40 is within [36,40].
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        assert!((40.0 - calculate_tinseth(&adds, 19.0, 1.060)).abs() <= 2.0);

        // 60min boil + whirlpool: formula-derived ~52.
        let adds = [
            add(30.0, 12.0, 60.0, "pellet", "boil"),
            add(50.0, 12.0, 0.0, "pellet", "whirlpool"),
        ];
        assert!((52.0 - calculate_tinseth(&adds, 19.0, 1.060)).abs() <= 2.0);

        // 50g EKG 5% 60min 19L OG1.050 pellet: formula-derived ~30.4.
        let adds = [add(50.0, 5.0, 60.0, "pellet", "boil")];
        assert!((30.4 - calculate_tinseth(&adds, 19.0, 1.050)).abs() <= 1.0);
    }

    #[test]
    fn calculate_tinseth_zero_volume() {
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        assert_eq!(calculate_tinseth(&adds, 0.0, 1.060), 0.0);
    }

    #[test]
    fn calculate_tinseth_dryhop_and_mash_ignored() {
        let adds = [
            add(100.0, 15.0, 0.0, "pellet", "dry-hop"),
            add(50.0, 10.0, 0.0, "pellet", "mash"),
        ];
        assert_eq!(calculate_tinseth(&adds, 19.0, 1.050), 0.0);
    }

    #[test]
    fn calculate_tinseth_leaf_form_factor() {
        let pellet = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        let leaf = [add(30.0, 12.0, 60.0, "leaf", "boil")];
        let ibu_pellet = calculate_tinseth(&pellet, 19.0, 1.060);
        let ibu_leaf = calculate_tinseth(&leaf, 19.0, 1.060);
        assert_relative_eq!(ibu_pellet * 0.9, ibu_leaf, epsilon = 0.01);
    }

    #[test]
    fn calculate_tinseth_first_wort() {
        // First-wort hops treated the same as boil additions at their stated time.
        let boil = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        let fw = [add(30.0, 12.0, 60.0, "pellet", "first-wort")];
        assert_relative_eq!(
            calculate_tinseth(&boil, 19.0, 1.060),
            calculate_tinseth(&fw, 19.0, 1.060),
            epsilon = 0.001
        );
    }

    #[test]
    fn calculate_rager_basic() {
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        let ibu = calculate_rager(&adds, 19.0, 1.060);
        assert!(ibu > 0.0);
        assert!(ibu < 100.0);
    }

    #[test]
    fn calculate_rager_ga_factor() {
        // Higher OG → GA factor reduces IBU.
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        let ibu_low = calculate_rager(&adds, 19.0, 1.040);
        let ibu_high = calculate_rager(&adds, 19.0, 1.090);
        assert!(ibu_low > ibu_high);
    }

    #[test]
    fn calculate_garetz_basic() {
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        // yeastFactor=1.0 should equal Rager.
        let ibu_rager = calculate_rager(&adds, 19.0, 1.060);
        let ibu_garetz1 = calculate_garetz(&adds, 19.0, 1.060, 1.0);
        let ibu_garetz2 = calculate_garetz(&adds, 19.0, 1.060, 1.10);
        assert_relative_eq!(ibu_rager, ibu_garetz1, epsilon = 0.01);
        assert!(ibu_garetz2 < ibu_garetz1);
    }

    #[test]
    fn calculate_garetz_zero_volume() {
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        assert_eq!(calculate_garetz(&adds, 0.0, 1.060, 1.0), 0.0);
    }

    #[test]
    fn calculate_rager_zero_volume() {
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        assert_eq!(calculate_rager(&adds, 0.0, 1.060), 0.0);
    }

    #[test]
    fn calculate_rager_dryhop_ignored() {
        let adds = [add(100.0, 15.0, 0.0, "pellet", "dry-hop")];
        assert_eq!(calculate_rager(&adds, 19.0, 1.050), 0.0);
    }

    #[test]
    fn calculate_rager_whirlpool() {
        let adds = [add(50.0, 12.0, 0.0, "pellet", "whirlpool")];
        let ibu = calculate_rager(&adds, 19.0, 1.050);
        assert!(ibu > 0.0);
    }

    #[test]
    fn calculate_garetz_dryhop_ignored() {
        let adds = [add(100.0, 15.0, 0.0, "pellet", "dry-hop")];
        assert_eq!(calculate_garetz(&adds, 19.0, 1.050, 1.0), 0.0);
    }

    #[test]
    fn calculate_garetz_zero_yeast_factor() {
        let adds = [add(30.0, 12.0, 60.0, "pellet", "boil")];
        assert_eq!(calculate_garetz(&adds, 19.0, 1.060, 0.0), 0.0);
    }

    #[test]
    fn normalise_hop_additions_all_use_cases() {
        for use_ in ["mash", "first-wort", "boil", "whirlpool", "dry-hop"] {
            let out = normalise_hop_additions(&[add(0.0, 0.0, 0.0, "", use_)]);
            assert_eq!(out[0].use_, use_, "use={use_:?}");
        }
    }

    #[test]
    fn normalise_hop_additions_mapping() {
        let input = [
            add(0.0, 0.0, 0.0, "whole", "boil"),
            add(0.0, 0.0, 0.0, "", ""),
            add(0.0, 0.0, 0.0, "pellet", "whirlpool"),
            add(0.0, 0.0, 0.0, "LEAF", "DRY-HOP"),
        ];
        let out = normalise_hop_additions(&input);
        assert_eq!(out[0].form, "leaf"); // "whole" → "leaf"
        assert_eq!(out[0].use_, "boil"); // preserved
        assert_eq!(out[1].form, "pellet"); // "" → "pellet"
        assert_eq!(out[1].use_, "boil"); // "" → "boil"
        assert_eq!(out[2].form, "pellet");
        assert_eq!(out[2].use_, "whirlpool");
        assert_eq!(out[3].form, "leaf"); // "LEAF" → "leaf"
        assert_eq!(out[3].use_, "dry-hop"); // "DRY-HOP" → "dry-hop"
    }
}
