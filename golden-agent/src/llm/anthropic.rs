use crate::{
    chat::{ChatModel, ChatStream},
    error::Error,
    request::ChatRequest,
    response::ChatResponse,
};
use async_trait::async_trait;

/// Commonly used Anthropic models (placeholder; list not guaranteed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnthropicModel {
    Claude35Sonnet,
    Claude35Haiku,
    Custom(String),
}

impl AnthropicModel {
    pub fn as_str(&self) -> &str {
        match self {
            AnthropicModel::Claude35Sonnet => "claude-3-5-sonnet-latest",
            AnthropicModel::Claude35Haiku => "claude-3-5-haiku-latest",
            AnthropicModel::Custom(s) => s,
        }
    }
}

impl Default for AnthropicModel {
    fn default() -> Self {
        AnthropicModel::Claude35Sonnet
    }
}

/// The Anthropic provider.
///
/// Currently a placeholder; Anthropic uses a protocol distinct from OpenAI and
/// requires its own message translation.
#[derive(Clone)]
pub struct AnthropicLlm {
    api_key: String,
}

impl AnthropicLlm {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl ChatModel for AnthropicLlm {
    async fn chat(&self, _request: &ChatRequest) -> Result<ChatResponse, Error> {
        todo!("Anthropic protocol not yet implemented")
    }

    fn chat_stream(&self, _request: &ChatRequest) -> ChatStream<'_> {
        Box::pin(async_stream::stream! {
            yield Err(Error::Stream("Anthropic streaming not yet implemented".to_string()));
        })
    }
}
