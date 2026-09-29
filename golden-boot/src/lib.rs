//! GoldenBoot is an annotation-driven web framework built on Axum.
//!
//! It provides a concise application entry point while keeping the underlying
//! runtime and HTTP server configuration out of application code.
//!
//! # Example
//!
//! ```ignore
//! use golden_boot::golden_boot_application;
//!
//! #[golden_boot_application(port = 8080)]
//! async fn main() {
//!     println!("initializing application");
//! }
//! ```

#[doc(inline)]
pub use golden_kernel::{
    ApiResponse, ApplicationError, Page, PaginationError, RequestEntity, RequestEntityError,
    ResponseEntity, ResponseEntityBuilder, ResponseEntityError, Validate, ValidationErrors,
};

#[doc(inline)]
pub use golden_macros::{
    RequestEntity, connect_mapping, delete_mapping, get_mapping, golden_boot_application,
    head_mapping, options_mapping, patch_mapping, post_mapping, put_mapping, trace_mapping,
};

#[doc(hidden)]
pub mod __private {
    pub use golden_kernel::{RouteDefinition, create_router, run, run_fallible};

    pub use golden_kernel::__private::{axum, inventory, serde, validator};
}

pub use golden_kernel::web::{
    Body, BodyDataStream, Bytes, DefaultBodyLimit, Event, Form, HeaderMap, HeaderName, HeaderValue,
    IntoResponse, Json, KeepAlive, Method, Multipart, Path, Query, Request, Response, Sse, State,
    StatusCode,
};

pub mod header {
    pub use golden_kernel::header::*;
}

pub mod multipart {
    pub use golden_kernel::multipart::*;
}

pub mod sse {
    pub use golden_kernel::sse::*;
}
