mod application;
mod error;
pub mod request;
pub mod response;
mod routing;

pub use application::DEFAULT_PORT;

#[doc(hidden)]
pub use application::{run, run_fallible};
pub use error::ApplicationError;

pub use response::{
    ApiResponse, Page, PaginationError, ResponseEntity, ResponseEntityBuilder, ResponseEntityError,
};

#[doc(hidden)]
pub use routing::{RouteDefinition, create_router};

#[doc(hidden)]
pub mod __private {
    pub use axum;
    pub use inventory;
    pub use serde;
    pub use validator;
}

/// Common web types used by GoldenBoot handlers.
pub mod web {
    pub use axum::{
        Form, Json,
        extract::{Path, Query, Request, State},
        http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
        response::{IntoResponse, Response},
    };
}

pub use request::{RequestEntity, RequestEntityError};

pub mod header {
    pub use axum::http::header::*;
}

pub use validator::{Validate, ValidationErrors};
