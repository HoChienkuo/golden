use axum::http::header::{InvalidHeaderName, InvalidHeaderValue};

/// An error produced while building a response.
#[derive(Debug, thiserror::Error)]
pub enum ResponseEntityError {
    /// The HTTP header name is invalid.
    #[error("invalid HTTP header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),

    /// The HTTP header value is invalid.
    #[error("invalid HTTP header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}

/// An invalid pagination argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PaginationError {
    /// Page numbers are one-based.
    #[error("page must be greater than zero")]
    InvalidPage,

    /// Page size must contain at least one item.
    #[error("page size must be greater than zero")]
    InvalidSize,
}
