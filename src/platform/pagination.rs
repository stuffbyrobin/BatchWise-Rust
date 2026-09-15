//! Shared pagination helpers for all list endpoints.
//!
//! This module consolidates pagination utilities that were duplicated in many
//! modules. The primary motivations were:
//! - Uncapped page sizes (allowing arbitrarily large page sizes)
//! - Overflow risk in `(page - 1) * page_size` when page is very large
//!   (e.g., `i64::MAX`), which can panic or cause undefined behavior.
//!
//! All list endpoints should use these helpers instead of reimplementing
//! clamping and offset calculation.

use serde::Serialize;

/// Default number of items per page when none is specified or when the
/// provided value is invalid.
pub const DEFAULT_PAGE_SIZE: i64 = 20;

/// Maximum allowed page size to prevent unbounded result sets.
pub const MAX_PAGE_SIZE: i64 = 100;

/// Clamps page and page_size to valid values using the module-level defaults.
///
/// - `page < 1` becomes `1`
/// - `page_size < 1` becomes `DEFAULT_PAGE_SIZE`
/// - `page_size > MAX_PAGE_SIZE` becomes `MAX_PAGE_SIZE`
///
/// See [`clamp_with`] for a version with customizable defaults.
pub fn clamp(page: i64, page_size: i64) -> (i64, i64) {
    clamp_with(page, page_size, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
}

/// Clamps page and page_size to valid values with custom defaults.
///
/// - `page < 1` becomes `1`
/// - `page_size < 1` becomes `default_size`
/// - `page_size > max_size` becomes `max_size`
pub fn clamp_with(page: i64, page_size: i64, default_size: i64, max_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 {
        default_size
    } else if page_size > max_size {
        max_size
    } else {
        page_size
    };
    (page, page_size)
}

/// Computes the SQL OFFSET value from page and page_size.
///
/// Uses saturating arithmetic so that extreme values like `page = i64::MAX`
/// cannot overflow or panic. The result is always >= 0.
pub fn offset(page: i64, page_size: i64) -> i64 {
    page.saturating_sub(1).saturating_mul(page_size)
}

/// Generic paginated response envelope.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Total number of items across all pages.
    pub total: i64,
    /// Current page number (1-indexed).
    pub page: i64,
    /// Number of items per page.
    pub page_size: i64,
    /// Total number of pages available.
    pub total_pages: i64,
}

impl<T> Page<T> {
    /// Builds a page, computing `total_pages`.
    ///
    /// `total_pages` is computed as `ceil(total / page_size)`, which is
    /// equivalent to `(total + page_size - 1) / page_size` for positive
    /// integers. Returns 0 if `page_size <= 0` or `total <= 0`.
    pub fn new(items: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 && total > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        Page {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_defaults() {
        assert_eq!(clamp(0, 0), (1, DEFAULT_PAGE_SIZE));
    }

    #[test]
    fn clamp_caps() {
        assert_eq!(clamp(3, 1000), (3, MAX_PAGE_SIZE));
    }

    #[test]
    fn clamp_with_custom() {
        assert_eq!(clamp_with(0, 0, 10, 50), (1, 10));
        assert_eq!(clamp_with(2, 200, 10, 50), (2, 50));
        assert_eq!(clamp_with(5, 25, 10, 50), (5, 25));
    }

    #[test]
    fn offset_first_page() {
        assert_eq!(offset(1, 20), 0);
    }

    #[test]
    fn offset_third_page() {
        assert_eq!(offset(3, 20), 40);
    }

    #[test]
    fn offset_max_page_no_panic() {
        let result = offset(i64::MAX, 100);
        assert!(result >= 0);
        assert_eq!(result, i64::MAX);
    }

    #[test]
    fn page_total_pages_ceil() {
        let p = Page::<()>::new(vec![], 45, 1, 20);
        assert_eq!(p.total_pages, 3);
    }

    #[test]
    fn page_total_pages_zero_total() {
        let p = Page::<()>::new(vec![], 0, 1, 20);
        assert_eq!(p.total_pages, 0);
    }

    #[test]
    fn page_total_pages_zero_page_size() {
        let p = Page::<()>::new(vec![], 45, 1, 0);
        assert_eq!(p.total_pages, 0);
    }
}
