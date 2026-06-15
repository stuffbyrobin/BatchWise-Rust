//! The 14 UK major allergen vocabulary and utilities.
//!
//! Port of the Go `pkg/allergen` package.

use std::collections::BTreeSet;

/// The canonical set of 14 UK major allergen tokens (lowercase).
pub const VOCABULARY: [&str; 14] = [
    "gluten",
    "wheat",
    "barley",
    "rye",
    "oats",
    "sulphites",
    "nuts",
    "peanuts",
    "soya",
    "milk",
    "celery",
    "mustard",
    "sesame",
    "lupin",
];

/// Returns an error message if any token is not in [`VOCABULARY`].
/// Tokens must already be normalised (lowercase, trimmed).
pub fn validate(tokens: &[String]) -> Result<(), String> {
    for t in tokens {
        if !VOCABULARY.contains(&t.as_str()) {
            return Err(format!(
                "unknown allergen token {t:?}; valid tokens: gluten, wheat, barley, rye, oats, \
                 sulphites, nuts, peanuts, soya, milk, celery, mustard, sesame, lupin"
            ));
        }
    }
    Ok(())
}

/// Lowercases and trims whitespace from each token.
pub fn normalise(tokens: &[String]) -> Vec<String> {
    tokens.iter().map(|t| t.trim().to_lowercase()).collect()
}

/// Returns the sorted, deduplicated union of two allergen slices.
pub fn union(a: &[String], b: &[String]) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.extend(a.iter().cloned());
    set.extend(b.iter().cloned());
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_known_rejects_unknown() {
        assert!(validate(&["gluten".into(), "milk".into()]).is_ok());
        assert!(validate(&["unicorn".into()]).is_err());
    }

    #[test]
    fn normalise_lowercases_and_trims() {
        assert_eq!(
            normalise(&["  Gluten ".into(), "MILK".into()]),
            vec!["gluten", "milk"]
        );
    }

    #[test]
    fn union_is_sorted_and_deduped() {
        assert_eq!(
            union(
                &["milk".into(), "gluten".into()],
                &["gluten".into(), "soya".into()]
            ),
            vec!["gluten", "milk", "soya"]
        );
    }
}
