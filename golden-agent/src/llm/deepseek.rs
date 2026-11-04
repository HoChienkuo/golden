use crate::error::Error;
use crate::llm::chat::{ChatModel, ChatRequest, ChatResponse, ChatStream, Reasoning};
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
    /// All non-deprecated built-in models, excluding custom model ids.
    pub const BUILT_IN: &'static [Self] = &[Self::Flash, Self::V4Pro];

    /// Iterates over all non-deprecated built-in models.
    pub fn built_in() -> impl ExactSizeIterator<Item = &'static Self> {
        Self::BUILT_IN.iter()
    }

    pub fn as_str(&self) -> &str {
        match self {
            DeepSeekModel::Flash => "deepseek-flash",
            DeepSeekModel::V4Pro => "deepseek-v4-pro",
            DeepSeekModel::Custom(s) => s,
        }
    }
}

/// The DeepSeek provider.
///
/// Thinking mode is controlled by the request's `thinking` object
/// (`{"type": "enabled" | "disabled"}`), with `reasoning_effort`
/// (`low` / `high` / `max`) selecting how hard the model thinks. Thinking is on
/// by default with effort `high`. See
/// <https://api-docs.deepseek.com/zh-cn/guides/thinking_mode>.
///
/// The chain of thought comes back as `reasoning_content`, which surfaces as
/// [`ChatResponse::reasoning`](crate::ChatResponse::reasoning) for non-streaming
/// calls and [`ChatEvent::Reasoning`](crate::ChatEvent::Reasoning) chunks when
/// streaming.
///
/// When a request carries `tools`, every earlier turn's `reasoning_content`
/// must be sent back or the API answers with `400`. This crate always echoes it
/// with the assistant message, and the agent keeps it on the messages it
/// persists, so tool-calling loops stay valid.
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

    /// Switches thinking mode on or off for a base request.
    ///
    /// `true` sends `thinking: {"type": "enabled"}`, `false` sends
    /// `thinking: {"type": "disabled"}`. Leaving the request untouched lets the
    /// API decide, which means thinking is on. Combine with
    /// [`Reasoning::Effort`] to control how hard the model thinks.
    ///
    /// ```
    /// use golden_agent::{ChatRequest, DeepSeekLlm, DeepSeekModel, Reasoning};
    ///
    /// let on = DeepSeekLlm::thinking(ChatRequest::new(DeepSeekModel::Flash.as_str()), true);
    /// let hard = on.reasoning(Reasoning::Effort(golden_agent::ReasoningEffort::High));
    ///
    /// assert_eq!(hard.reasoning, Some(Reasoning::Effort(golden_agent::ReasoningEffort::High)));
    /// ```
    pub fn thinking(request: ChatRequest, enabled: bool) -> ChatRequest {
        request.reasoning(if enabled {
            Reasoning::Enabled
        } else {
            Reasoning::Disabled
        })
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
