//! Field formatting helpers for label rendering.
//!
//! Port of the Go `pkg/labelkit/format.go`. `title_case` matches the Go
//! implementation: ASCII-only case mapping (first char upper, rest lower).

use chrono::NaiveDate;

/// Title-cases a string: first char uppercase, the rest lowercase (ASCII only;
/// non-ASCII chars are left unchanged, matching the Go `toUpper`/`toLower`).
fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        if i == 0 {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push(c.to_ascii_lowercase());
        }
    }
    out
}

/// Formats a date as UK day/month/year (`02/01/2006` → `%d/%m/%Y`).
pub fn format_best_before(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

/// Returns a comma-separated, title-cased string of allergens.
pub fn format_allergens(allergens: &[String]) -> String {
    allergens
        .iter()
        .map(|a| title_case(a))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allergens_title_cased_and_joined() {
        assert_eq!(format_allergens(&[]), "");
        assert_eq!(
            format_allergens(&["gluten".into(), "MILK".into()]),
            "Gluten, Milk"
        );
    }

    #[test]
    fn best_before_uk_format() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        assert_eq!(format_best_before(d), "09/03/2026");
    }
}
