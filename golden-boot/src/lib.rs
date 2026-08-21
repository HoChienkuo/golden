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
pub use golden_core::ApplicationError;

#[doc(inline)]
pub use golden_macros::golden_boot_application;

#[doc(hidden)]
pub mod __private {
    pub use golden_core::run;
}
