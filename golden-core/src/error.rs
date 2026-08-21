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
}