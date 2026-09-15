//! SQL LIKE pattern utilities.
//!
//! 15 repository sites build LIKE patterns as format!("%{}%", term) without escaping,
//! so a user-supplied '%' or '_' acts as a wildcard.
//! In PostgreSQL the default LIKE escape character is a backslash,
//! so escaping the term is sufficient and no ESCAPE clause is required.

/// Escapes special LIKE characters in a term.
///
/// Backslashes, percent signs, and underscores are prefixed with a backslash.
/// The backslash is escaped first to avoid double-escaping.
///
/// # Examples
///
/// ```
/// use platform::sql::escape_like;
/// assert_eq!(escape_like("hello"), "hello");
/// assert_eq!(escape_like("50%"), "50\\%");
/// assert_eq!(escape_like("a_b"), "a\\_b");
/// assert_eq!(escape_like("\\"), "\\\\");
/// ```
pub fn escape_like(term: &str) -> String {
    let mut result = String::with_capacity(term.len() * 2);
    for c in term.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '%' => result.push_str("\\%"),
            '_' => result.push_str("\\_"),
            _ => result.push(c),
        }
    }
    result
}

/// Returns a LIKE pattern that matches strings containing the term.
///
/// The term is escaped and wrapped with `%` on both sides.
///
/// # Examples
///
/// ```
/// use platform::sql::like_contains;
/// assert_eq!(like_contains("hello"), "%hello%");
/// assert_eq!(like_contains("50%"), "%50\\%%");
/// assert_eq!(like_contains("a_b"), "%a\\_b%");
/// ```
pub fn like_contains(term: &str) -> String {
    format!("%{}%", escape_like(term))
}

/// Returns a LIKE pattern that matches strings starting with the term.
///
/// The term is escaped and wrapped with `%` only at the end.
///
/// # Examples
///
/// ```
/// use platform::sql::like_prefix;
/// assert_eq!(like_prefix("hello"), "hello%");
/// assert_eq!(like_prefix("50%"), "50\\%%");
/// assert_eq!(like_prefix("\\"), "\\\\%");
/// ```
pub fn like_prefix(term: &str) -> String {
    format!("{}%", escape_like(term))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_term_unchanged_inside_wildcards() {
        assert_eq!(like_contains("hello"), "%hello%");
    }

    #[test]
    fn percent_sign_escaped() {
        assert_eq!(like_contains("50%"), "%50\\%%");
    }

    #[test]
    fn underscore_escaped() {
        assert_eq!(like_contains("a_b"), "%a\\_b%");
    }

    #[test]
    fn backslash_doubled() {
        assert_eq!(escape_like("\\"), "\\\\");
        assert_eq!(like_contains("\\"), "%\\\\%");
    }

    #[test]
    fn like_prefix_no_leading_percent() {
        assert_eq!(like_prefix("test"), "test%");
    }
}
