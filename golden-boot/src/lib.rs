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
pub use golden_core::{
    ApplicationError,
    ResponseEntity,
    ResponseEntityBuilder,
    ResponseEntityError,
    ApiResponse,
    Page,
    PaginationError,
    Validate,
    ValidationErrors,
    RequestEntity,
    RequestEntityError,
};

#[doc(inline)]
pub use golden_macros::{
    connect_mapping,
    delete_mapping,
    get_mapping,
    golden_boot_application,
    head_mapping,
    options_mapping,
    patch_mapping,
    post_mapping,
    put_mapping,
    trace_mapping,
    RequestEntity,
};

#[doc(hidden)]
pub mod __private {
    pub use golden_core::{
        run,
        RouteDefinition,
    };

    pub use golden_core::__private::{
        axum,
        inventory,
        serde,
        validator,
    };
}

pub use golden_core::web::{
    Form,
    HeaderMap,
    HeaderName,
    HeaderValue,
    IntoResponse,
    Json,
    Method,
    Path,
    Query,
    Request,
    Response,
    StatusCode,
};

pub mod header {
    pub use golden_core::header::*;
}
