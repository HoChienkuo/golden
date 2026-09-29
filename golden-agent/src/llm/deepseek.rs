use crate::error::Error;
use crate::llm::chat::{ChatModel, ChatRequest, ChatResponse, ChatStream};
use crate::llm::http::HttpConfig;
use crate::llm::openai::OpenAiLlm;
use async_trait::async_trait;

/// Commonly used DeepSeek models.
/// This list may fall behind; use `Custom` for unlisted models.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DeepSeekModel {
    #[default]
    Flash,
    V4Pro,
    Custom(String),
}

impl DeepSeekModel {
    pub fn as_str(&self) -> &str {
        match self {
            DeepSeekModel::Flash => "deepseek-flash",
            DeepSeekModel::V4Pro => "deepseek-v4-pro",
            DeepSeekModel::Custom(s) => s,
        }
    }
}

/// The DeepSeek provider.
#[derive(Clone)]
pub struct DeepSeekLlm(OpenAiLlm);

impl DeepSeekLlm {
    pub const BASE_URL: &str = "https://api.deepseek.com";

    /// Constructs from the `DEEPSEEK_API_KEY` environment variable.
    pub fn from_env() -> Self {
        let key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set");
        Self::new(key)
    }

    /// Constructs with an explicit API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self(OpenAiLlm::new(Self::BASE_URL, api_key))
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
impl ChatModel for DeepSeekLlm {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        self.0.chat(request).await
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        self.0.chat_stream(request)
    }
}
