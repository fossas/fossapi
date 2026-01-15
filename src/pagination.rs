//! Pagination utilities for FOSSA API responses.

use serde::{Deserialize, Serialize};

/// A page of results from the FOSSA API.
#[derive(Debug, Clone, Serialize)]
#[serde(bound = "T: Serialize")]
pub struct Page<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Total number of items across all pages (if known).
    pub total: Option<u64>,
    /// Current page number (1-indexed).
    pub page: u32,
    /// Number of items per page.
    pub count: u32,
    /// Whether there are more pages.
    pub has_more: bool,
}

impl<T> Page<T> {
    /// Create a new page from items and pagination info.
    #[must_use]
    pub fn new(items: Vec<T>, page: u32, count: u32, total: Option<u64>) -> Self {
        let has_more = match total {
            Some(t) => (u64::from(page) * u64::from(count)) < t,
            None => items.len() >= count as usize,
        };
        Self {
            items,
            total,
            page,
            count,
            has_more,
        }
    }

    /// Map the items to a different type.
    #[must_use]
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            page: self.page,
            count: self.count,
            has_more: self.has_more,
        }
    }

    /// Returns true if this page has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items on this page.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns an iterator over the items in this page.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }
}

impl<T> IntoIterator for Page<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Page<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

/// Query parameters for paginated requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Number of items per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

impl PaginationParams {
    /// Create pagination params for a specific page.
    #[must_use]
    pub fn for_page(page: u32, count: u32) -> Self {
        Self {
            page: Some(page),
            count: Some(count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_has_more_with_total() {
        // Page 1 of 3 (total 250, 100 per page)
        let page: Page<i32> = Page::new(vec![1; 100], 1, 100, Some(250));
        assert!(page.has_more);

        // Page 3 of 3
        let page: Page<i32> = Page::new(vec![1; 50], 3, 100, Some(250));
        assert!(!page.has_more);
    }

    #[test]
    fn test_page_has_more_without_total() {
        // Full page suggests more
        let page: Page<i32> = Page::new(vec![1; 100], 1, 100, None);
        assert!(page.has_more);

        // Partial page means no more
        let page: Page<i32> = Page::new(vec![1; 50], 1, 100, None);
        assert!(!page.has_more);
    }

    #[test]
    fn test_page_map() {
        let page = Page::new(vec![1, 2, 3], 1, 100, Some(3));
        let mapped = page.map(|x| x * 2);
        assert_eq!(mapped.items, vec![2, 4, 6]);
        assert_eq!(mapped.page, 1);
    }
}
