//! Shared HTTP reliability: request timeouts and automatic retries.

use std::future::Future;
use std::time::Duration;

use crate::error::Error;

/// The longest delay exponential backoff will ever wait between retries.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// HTTP reliability settings shared by every provider.
///
/// The defaults retry transient failures twice and impose no timeout, matching
/// the behavior of the underlying `reqwest` client. Build a value with the
/// chained setters and pass it to a provider's `with_http_config`:
///
/// ```
/// use std::time::Duration;
///
/// use golden_agent::{DeepSeekLlm, HttpConfig};
///
/// let config = HttpConfig::new()
///     .timeout(Duration::from_secs(60))
///     .connect_timeout(Duration::from_secs(10))
///     .max_retries(3);
///
/// let llm = DeepSeekLlm::new("api-key").with_http_config(config);
/// ```
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// A total timeout applied to each non-streaming request attempt.
    ///
    /// Streaming requests ignore this value, because a long generation is
    /// expected to keep the response body open. Configure
    /// [`connect_timeout`](Self::connect_timeout) to bound their connection.
    pub timeout: Option<Duration>,
    /// A timeout applied while establishing a connection, for every request.
    pub connect_timeout: Option<Duration>,
    /// The number of retries after the first attempt for retryable failures.
    ///
    /// Transport errors, HTTP `429`, and `5xx` responses are retried. A value
    /// of `0` disables retries.
    pub max_retries: u32,
    /// The base delay for exponential backoff between retries.
    pub backoff: Duration,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            connect_timeout: None,
            max_retries: 2,
            backoff: Duration::from_millis(500),
        }
    }
}

impl HttpConfig {
    /// Creates the default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the total timeout applied to non-streaming requests.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the connection-establishment timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Sets the number of retries after the first attempt.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the base delay for exponential backoff.
    pub fn backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    /// Builds the `reqwest` client implied by this configuration.
    pub(crate) fn build_client(&self) -> reqwest::Client {
        let mut builder = reqwest::Client::builder();
        if let Some(connect_timeout) = self.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }
        builder.build().expect("failed to build the HTTP client")
    }
}

/// Runs `attempt` until it succeeds or fails in a non-retryable way.
///
/// Retryable failures are retried up to [`HttpConfig::max_retries`] times, with
/// exponential backoff derived from [`HttpConfig::backoff`].
pub(crate) async fn with_retry<T, F, Fut>(config: &HttpConfig, mut attempt: F) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut retries = 0;

    loop {
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if retries >= config.max_retries || !is_retryable(&error) {
                    return Err(error);
                }
                tokio::time::sleep(backoff_delay(config.backoff, retries)).await;
                retries += 1;
            }
        }
    }
}

/// Whether a failed attempt is worth retrying.
fn is_retryable(error: &Error) -> bool {
    match error {
        // Transport failures are transient, except for requests that could not
        // be built or responses that could not be decoded.
        Error::Request(error) => !error.is_builder() && !error.is_decode(),
        Error::Api { status, .. } => {
            *status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
        }
        // Streaming, tool, and agent errors are not transport failures.
        _ => false,
    }
}

/// Exponential backoff for the given zero-based retry index, capped at
/// [`MAX_BACKOFF`].
fn backoff_delay(base: Duration, retry: u32) -> Duration {
    let factor = 1u32 << retry.min(6);
    base.saturating_mul(factor).min(MAX_BACKOFF)
}
