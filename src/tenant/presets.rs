//! Feature-flag presets per tier.
//!
//! Port of the Go `internal/tenant/presets.go`. Returns a fresh map each call.

use std::collections::HashMap;

/// Returns the `feature_flags` map for a new tenant at the given tier.
pub fn preset_for_tier(tier: &str) -> HashMap<String, bool> {
    let mut flags = HashMap::new();
    match tier {
        "home" => {
            for k in [
                "inventory",
                "recipes",
                "batches",
                "calendar",
                "yeastkinetics",
                "library",
                "water",
                "yeast_banking",
                "fermentation",
            ] {
                flags.insert(k.to_string(), true);
            }
        }
        "pro" => {
            flags = preset_for_tier("home");
            for k in [
                "tracking",
                "reporting",
                "sales",
                "duty",
                "allergens",
                "labels",
                "packaging",
                "traceability",
                "equipment_maintenance",
            ] {
                flags.insert(k.to_string(), true);
            }
        }
        "enterprise" => {
            flags = preset_for_tier("pro");
        }
        _ => {}
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_has_core_flags() {
        let f = preset_for_tier("home");
        assert_eq!(f.get("inventory"), Some(&true));
        assert!(!f.contains_key("reporting"));
    }

    #[test]
    fn pro_extends_home() {
        let f = preset_for_tier("pro");
        assert_eq!(f.get("inventory"), Some(&true));
        assert_eq!(f.get("reporting"), Some(&true));
        assert_eq!(f.get("sales"), Some(&true));
    }

    #[test]
    fn enterprise_equals_pro() {
        assert_eq!(preset_for_tier("enterprise"), preset_for_tier("pro"));
    }

    #[test]
    fn unknown_tier_empty() {
        assert!(preset_for_tier("nope").is_empty());
    }
}
