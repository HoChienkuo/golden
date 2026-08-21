mod application;
mod error;
pub mod response;
mod routing;

pub use application::{DEFAULT_PORT, run};
pub use error::ApplicationError;

pub use response::{
    ApiResponse,
    Page,
    PaginationError,
    ResponseEntity,
    ResponseEntityBuilder,
    ResponseEntityError,
};

#[doc(hidden)]
pub use routing::RouteDefinition;

#[doc(hidden)]
pub mod __private {
    pub use axum;
    pub use inventory;
}

/// Common web types used by GoldenBoot handlers.
pub mod web {
    pub use axum::{
        Form, Json,
        extract::{Path, Query, Request},
        http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
        response::{IntoResponse, Response},
    };
}
