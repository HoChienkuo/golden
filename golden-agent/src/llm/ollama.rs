use crate::error::Error;
use crate::llm::chat::{ChatModel, ChatRequest, ChatResponse, ChatStream};
use crate::llm::http::HttpConfig;
use crate::llm::openai::OpenAiLlm;
use async_trait::async_trait;

/// The Ollama provider, speaking Ollama's OpenAI-compatible protocol.
///
/// Ollama serves local models behind an OpenAI-compatible
/// `/v1/chat/completions` endpoint. No authentication is required by default,
/// so the api key may be empty.
///
/// ```ignore
/// use golden_agent::llm::OllamaLlm;
///
/// // Start the server with `ollama serve` and pull a model first:
/// //   ollama pull llama3.1
/// let llm = OllamaLlm::new();
/// ```
#[derive(Clone)]
pub struct OllamaLlm(OpenAiLlm);

impl OllamaLlm {
    /// Default base URL for a local Ollama server (`http://localhost:11434/v1`).
    pub const BASE_URL: &str = "http://localhost:11434/v1";

    /// Connects to a local Ollama server at the default port, without an api key.
    pub fn new() -> Self {
        Self::with_base_url(Self::BASE_URL)
    }

    /// Connects to an Ollama server at a custom base URL, without an api key.
    ///
    /// `base_url` should include the scheme and the `/v1` suffix, for example
    /// `http://remote-host:11434/v1`.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self(OpenAiLlm::new(base_url, ""))
    }

    /// Connects to an Ollama server at a custom base URL with an explicit api
    /// key, for servers configured to require one.
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

impl Default for OllamaLlm {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatModel for OllamaLlm {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        self.0.chat(request).await
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        self.0.chat_stream(request)
    }
}
