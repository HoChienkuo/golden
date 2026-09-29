//! Shared HTTP transport settings.

use std::time::Duration;

/// HTTP transport settings shared by every provider.
///
/// The defaults impose no timeout, matching the underlying `reqwest` client.
///
/// ```
/// use std::time::Duration;
///
/// use golden_agent::{DeepSeekLlm, HttpConfig};
///
/// let config = HttpConfig::new()
///     .timeout(Duration::from_secs(60))
///     .connect_timeout(Duration::from_secs(10));
///
/// let llm = DeepSeekLlm::new("api-key").with_http_config(config);
/// ```
#[derive(Debug, Clone, Default)]
pub struct HttpConfig {
    /// A total timeout applied to each non-streaming request.
    ///
    /// Streaming requests ignore this value, because a long generation is
    /// expected to keep the response body open. Configure
    /// [`connect_timeout`](Self::connect_timeout) to bound their connection.
    pub timeout: Option<Duration>,
    /// A timeout applied while establishing a connection, for every request.
    pub connect_timeout: Option<Duration>,
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

    /// Builds the `reqwest` client implied by this configuration.
    pub(crate) fn build_client(&self) -> reqwest::Client {
        let mut builder = reqwest::Client::builder();
        if let Some(connect_timeout) = self.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }
        builder.build().expect("failed to build the HTTP client")
    }

    /// Applies the configured total timeout, for non-streaming requests only.
    pub(crate) fn bound(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        }
    }
}
