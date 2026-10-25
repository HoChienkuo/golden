use crate::error::Error;
use crate::llm::chat::{ChatModel, ChatRequest, ChatResponse, ChatStream};
use crate::llm::http::HttpConfig;
use crate::llm::openai::OpenAiLlm;
use async_trait::async_trait;

/// The llama.cpp provider, speaking llama-server's OpenAI-compatible protocol.
///
/// llama.cpp runs local GGUF models behind an OpenAI-compatible
/// `/v1/chat/completions` endpoint. No authentication is required by default,
/// so the api key may be empty.
///
/// ```ignore
/// use golden_agent::llm::LlamaCppLlm;
///
/// // Connect to a server started with:
/// //   ./llama-server -m /path/to/model.gguf --port 8080
/// let llm = LlamaCppLlm::new("http://localhost:8080/v1");
/// ```
#[derive(Clone)]
pub struct LlamaCppLlm(OpenAiLlm);

impl LlamaCppLlm {
    /// Constructs from a base URL, without an api key.
    ///
    /// `base_url` should include the scheme and the `/v1` suffix, for example
    /// `http://localhost:8080/v1`. Use an empty `api_key` for servers that do
    /// not require authentication.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self(OpenAiLlm::new(base_url, ""))
    }

    /// Constructs from a base URL with an explicit api key, for servers
    /// configured to require one.
    pub fn with_api_key(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self(OpenAiLlm::new(base_url, api_key))
    }

    /// Replaces the HTTP reliability settings, rebuilding the underlying client.
    pub fn with_http_config(self, config: HttpConfig) -> Self {
        Self(self.0.with_http_config(config))
    }

    /// Exposes the underlying OpenAI protocol implementation.
    pub fn inner(&self) -> &OpenAiLlm {
        &self.0
    }
}

#[async_trait]
impl ChatModel for LlamaCppLlm {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        self.0.chat(request).await
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        self.0.chat_stream(request)
    }
}
