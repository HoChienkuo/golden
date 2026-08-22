use std::io;

/// An error that can occur while starting or running a GoldenBoot application.
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    /// The Tokio runtime could not be created.
    #[error("failed to create Tokio runtime: {0}")]
    Runtime(#[source] io::Error),

    /// The HTTP server could not bind to the configured address.
    ///
    /// This commonly occurs when the port is already in use or the process
    /// does not have permission to bind to the address.
    #[error("failed to bind HTTP server: {0}")]
    Bind(#[source] io::Error),

    /// The Axum HTTP server stopped because of an I/O error.
    #[error("HTTP server failed: {0}")]
    Serve(#[source] io::Error),

    /// Two handlers registered the same HTTP method and path.
    #[error(
        "duplicate route `{method} {path}`: \
         handlers `{first_handler}` and `{second_handler}`"
    )]
    DuplicateRoute {
        method: &'static str,
        path: &'static str,
        first_handler: &'static str,
        second_handler: &'static str,
    },

    /// A handler expects a different application state type.
    #[error(
        "handler `{handler}` expects application state `{expected}`, \
         but the application provides `{actual}`"
    )]
    StateTypeMismatch {
        handler: &'static str,
        expected: &'static str,
        actual: &'static str,
    },
}
