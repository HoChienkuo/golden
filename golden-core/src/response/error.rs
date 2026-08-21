use axum::http::header::{
    InvalidHeaderName,
    InvalidHeaderValue,
};

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