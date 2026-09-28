use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;

use crate::{error::Error, request::ChatRequest, response::ChatResponse};

/// A stream of incremental text tokens produced by a streaming chat completion.
pub type ChatStream<'a> = Pin<Box<dyn Stream<Item = Result<String, Error>> + Send + 'a>>;

/// A unified abstraction over chat-capable language models.
///
/// Every provider (OpenAI, DeepSeek, Ollama, Anthropic, ...) implements this
/// trait. Higher-level code depends only on [`ChatModel`] and remains agnostic
/// to the underlying provider and protocol.
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// Performs a single, non-streaming chat completion.
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error>;

    /// Performs a streaming chat completion, yielding text deltas.
    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_>;
}
