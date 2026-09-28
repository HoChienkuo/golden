use serde::Deserialize;
use thiserror::Error;

/// The error type for golden-agent.
#[derive(Debug, Error)]
pub enum Error {
    /// An HTTP transport error.
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The API returned an error status code.
    #[error("api error (status {status}): {message}")]
    Api {
        /// The HTTP status code.
        status: reqwest::StatusCode,
        /// The parsed error body, if any.
        body: Option<ApiErrorBody>,
        /// A human-readable error message.
        message: String,
    },

    /// A streaming response parsing error.
    #[error("invalid sse data: {0}")]
    Stream(String),
}

/// The OpenAI-style error response body.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

impl Error {
    /// Constructs an API error from a status code and optional body.
    pub(crate) fn api(status: reqwest::StatusCode, body: Option<ApiErrorBody>) -> Self {
        let message = body
            .as_ref()
            .map(|b| b.error.message.clone())
            .unwrap_or_else(|| "unknown api error".to_string());
        Error::Api {
            status,
            body,
            message,
        }
    }
}
