//! Parse and validate `?sort=` query strings for list endpoints.
//!
//! List endpoints accept `?sort=col[,col2]` with a leading `-` for DESC.
//! Each module declares a `const` allow-list mapping public names to trusted
//! SQL column expressions. Unknown names are rejected with a validation error.
//! The returned fragment is built only from allow-list values, never from
//! user input (SQL-injection safe by construction).

use crate::platform::errors::ApiError;

/// A mapping of public sort field names to trusted SQL column expressions.
///
/// Each module that supports sorting should define a `const ALLOWED: Allowed = &[...]`
/// with entries like `("order_date", "o.order_date")` to expose a public name that
/// maps to the actual SQL expression.
pub type Allowed = &'static [(&'static str, &'static str)];

/// Controls where NULL values appear in sorted results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nulls {
    /// Use the Postgres default: `NULLS LAST` for `ASC`, `NULLS FIRST` for `DESC`.
    Default,
    /// Append `NULLS LAST` to every sort field so rows with missing values
    /// always sort to the end, regardless of sort direction.
    Last,
}

/// Parse a sort specification string against an allow-list.
///
/// This is equivalent to `parse_with(spec, allowed, default, Nulls::Default)`.
///
/// # Arguments
///
/// * `spec` - A comma-separated list of field names, optionally prefixed with `-`
///   for descending order. Empty or whitespace-only strings use `default`.
/// * `allowed` - The allow-list mapping public names to SQL column expressions.
/// * `default` - The default sort specification to use when `spec` is empty
///   or produces no valid fields. This is itself a spec string (e.g., `-created_at`
///   or `"name"`) and is parsed the same way.
///
/// # Returns
///
/// A `Result` containing the SQL `ORDER BY` fragment, or a validation error
/// if any field name is not in the allow-list.
///
/// # Examples
///
/// * `spec="-name"` with allow-list containing `("name", "name")` \=> `"name DESC"`
/// * `spec="name,-created_at"` \=> `"name ASC, created_at DESC"`
pub fn parse(spec: &str, allowed: Allowed, default: &str) -> Result<String, ApiError> {
    parse_with(spec, allowed, default, Nulls::Default)
}

/// Parse a sort specification with explicit NULL handling.
///
/// This is the full parsing function; [`parse`] is a convenience wrapper
/// that uses `Nulls::Default`.
///
/// # Arguments
///
/// * `spec` - A comma-separated list of field names, optionally prefixed with `-`
///   for descending order. Empty or whitespace-only strings use `default`.
/// * `allowed` - The allow-list mapping public names to SQL column expressions.
/// * `default` - The default sort specification to use when `spec` is empty
///   or produces no valid fields. This is itself a spec string and is parsed
///   the same way.
/// * `nulls` - Controls NULL value placement (`Default` or `Last`).
///
/// # Returns
///
/// A `Result` containing the SQL `ORDER BY` fragment, or a validation error.
///
/// # Examples
///
/// * `spec="-name"`, `nulls=Nulls::Last` \=> `"name DESC NULLS LAST"`
/// * `spec=""` \=> parses `default` (e.g., if default is `-created_at` \=> `"created_at DESC"`)
/// * `spec="name,"` with allow-list \=> `"name ASC"` (trailing comma/empty segments ignored)
/// * `spec="unknown"` \=> `Err(ApiError::validation("sort", "invalid sort field: unknown"))`
pub fn parse_with(
    spec: &str,
    allowed: Allowed,
    default: &str,
    nulls: Nulls,
) -> Result<String, ApiError> {
    let spec = spec.trim();
    let spec_to_parse = if spec.is_empty() { default } else { spec };

    let mut parts = Vec::new();
    for field in spec_to_parse.split(',') {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, dir) = match field.strip_prefix('-') {
            Some(rest) => (rest, "DESC"),
            None => (field, "ASC"),
        };
        let col = allowed
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, c)| *c)
            .ok_or_else(|| ApiError::validation("sort", &format!("invalid sort field: {name}")))?;

        let mut fragment = format!("{col} {dir}");
        if nulls == Nulls::Last {
            fragment.push_str(" NULLS LAST");
        }
        parts.push(fragment);
    }

    if parts.is_empty() {
        // Only reachable when `spec` had no usable segments (e.g. ","); the
        // default is a module constant and must itself be non-empty.
        if spec_to_parse == default {
            return Err(ApiError::internal("empty default sort spec"));
        }
        return parse_with(default, allowed, default, nulls);
    }

    Ok(parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALLOWED: Allowed = &[("name", "name"), ("created_at", "created_at")];

    #[test]
    fn default_used_when_spec_empty() {
        let result = parse("", ALLOWED, "name").unwrap();
        assert_eq!(result, "name ASC");
    }

    #[test]
    fn single_desc_field() {
        let result = parse("-name", ALLOWED, "name").unwrap();
        assert_eq!(result, "name DESC");
    }

    #[test]
    fn multi_field() {
        let result = parse("name,-created_at", ALLOWED, "name").unwrap();
        assert_eq!(result, "name ASC, created_at DESC");
    }

    #[test]
    fn mapping_to_prefixed_column() {
        const PREFIXED: Allowed = &[("name", "s.name"), ("order_date", "o.order_date")];
        let result = parse("-order_date,name", PREFIXED, "name").unwrap();
        assert_eq!(result, "o.order_date DESC, s.name ASC");
    }

    #[test]
    fn unknown_field_returns_err() {
        let result = parse("unknown", ALLOWED, "name");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), "validation_error");
    }

    #[test]
    fn nulls_last_appends_to_every_field() {
        let result = parse_with("name,-created_at", ALLOWED, "name", Nulls::Last).unwrap();
        assert_eq!(result, "name ASC NULLS LAST, created_at DESC NULLS LAST");
    }
}
