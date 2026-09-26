use crate::response::error::PaginationError;
use serde::Serialize;

/// A one-based page of items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    items: Vec<T>,
    page: u32,
    size: u32,
    total: u64,
    total_pages: u64,
    has_next: bool,
    has_previous: bool,
}

impl<T> Page<T> {
    /// Creates a one-based page.
    ///
    /// Both `page` and `size` must be greater than zero.
    pub fn new(items: Vec<T>, page: u32, size: u32, total: u64) -> Result<Self, PaginationError> {
        if page == 0 {
            return Err(PaginationError::InvalidPage);
        }

        if size == 0 {
            return Err(PaginationError::InvalidSize);
        }

        let total_pages = total.div_ceil(u64::from(size));
        let has_previous = page > 1;
        let has_next = u64::from(page) < total_pages;

        Ok(Self {
            items,
            page,
            size,
            total,
            total_pages,
            has_next,
            has_previous,
        })
    }

    /// Returns the items in this page.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Returns mutable access to the page items.
    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }

    /// Consumes the page and returns its items.
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    /// Returns the one-based page number.
    pub fn page(&self) -> u32 {
        self.page
    }

    /// Returns the requested page size.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Returns the total number of items.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Returns the total number of pages.
    pub fn total_pages(&self) -> u64 {
        self.total_pages
    }

    /// Returns whether another page exists.
    pub fn has_next(&self) -> bool {
        self.has_next
    }

    /// Returns whether a previous page exists.
    pub fn has_previous(&self) -> bool {
        self.has_previous
    }

    /// Maps every item while preserving pagination metadata.
    pub fn map<U>(self, mut mapper: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(&mut mapper).collect(),
            page: self.page,
            size: self.size,
            total: self.total,
            total_pages: self.total_pages,
            has_next: self.has_next,
            has_previous: self.has_previous,
        }
    }
}
