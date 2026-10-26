use std::time::Duration;

use async_trait::async_trait;

use crate::error::Error;
use crate::llm::chat::{ChatRequest, ChatResponse};

use super::middleware::{Middleware, ModelHandler};

/// The longest delay exponential backoff will ever wait between retries.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// A middleware that retries failed model calls with exponential backoff.
///
/// Only transient failures are retried: transport errors, HTTP `429`, and `5xx`
/// responses. It wraps each model call, so it does not apply to
/// [`Agent::stream`](crate::Agent::stream), which bypasses the model-call chain.
///
/// ```
/// use golden_agent::Retry;
///
/// let retry = Retry::new().max_retries(3);
/// # let _ = retry;
/// ```
#[derive(Debug, Clone)]
pub struct Retry {
    max_retries: u32,
    backoff: Duration,
}

impl Default for Retry {
    fn default() -> Self {
        Self {
            max_retries: 2,
            backoff: Duration::from_millis(500),
        }
    }
}

impl Retry {
    /// Creates a retry middleware with the default policy: two retries after the
    /// first attempt, with a 500ms base backoff.
    pub fn new() -> Self {
        Self::default()
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
}

#[async_trait]
impl Middleware for Retry {
    async fn wrap_model_call(
        &self,
        request: ChatRequest,
        next: &dyn ModelHandler,
    ) -> Result<ChatResponse, Error> {
        let mut retries = 0;

        loop {
            match next.handle(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if retries >= self.max_retries || !is_retryable(&error) {
                        return Err(error);
                    }
                    tokio::time::sleep(backoff_delay(self.backoff, retries)).await;
                    retries += 1;
                }
            }
        }
    }
}

/// Whether a failed model call is worth retrying.
fn is_retryable(error: &Error) -> bool {
    match error {
        // Transport failures are transient, except for requests that could not
        // be built or responses that could not be decoded.
        Error::Request(error) => !error.is_builder() && !error.is_decode(),
        Error::Api { status, .. } => {
            *status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
        }
        _ => false,
    }
}

/// Exponential backoff for the given zero-based retry index, capped at
/// [`MAX_BACKOFF`].
fn backoff_delay(base: Duration, retry: u32) -> Duration {
    let factor = 1u32 << retry.min(6);
    base.saturating_mul(factor).min(MAX_BACKOFF)
}
