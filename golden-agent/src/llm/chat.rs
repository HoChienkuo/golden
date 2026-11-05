use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Error;
use crate::tool::ToolSpec;

/// An incremental event from a streaming chat completion.
///
/// Providers stream tool calls in fragments; each provider accumulates them and
/// emits a complete [`ChatEvent::ToolCall`] once the call is finished.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    /// A chunk of the model's reasoning (thinking) output.
    ///
    /// Only emitted when the request asked for reasoning and the provider
    /// streams it; it always arrives before the corresponding [`Self::Text`].
    Reasoning(String),
    /// An opaque signature for the streamed reasoning block.
    ///
    /// Providers such as Anthropic require this value to be returned unchanged
    /// when the reasoning block is included in a later request.
    ReasoningSignature(String),
    /// A chunk of assistant text.
    Text(String),
    /// A tool call the model finished streaming.
    ToolCall(ToolCall),
}

/// A stream of incremental events produced by a streaming chat completion.
pub type ChatStream<'a> = Pin<Box<dyn Stream<Item = Result<ChatEvent, Error>> + Send + 'a>>;

/// A unified abstraction over chat-capable language models.
///
/// The request and response types are provider-neutral; each provider translates
/// them into (and out of) its own wire format internally.
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// Performs a single, non-streaming chat completion.
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error>;

    /// Performs a streaming chat completion, yielding incremental events.
    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_>;
}

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A system instruction.
    System,
    /// A user turn.
    User,
    /// An assistant turn.
    Assistant,
    /// The result of a tool call.
    Tool,
}

/// The content of a message.
///
/// Content is either a plain string ([`Content::Text`]) or a list of typed parts
/// ([`Content::Parts`]) used for multimodal inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Content {
    /// Plain text content.
    Text(String),
    /// An ordered list of typed content parts.
    Parts(Vec<ContentPart>),
}

impl Content {
    /// Returns the first text of this content, if any.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Text(text) => Some(text),
            Content::Parts(parts) => parts.iter().find_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                ContentPart::ImageUrl { .. } => None,
            }),
        }
    }

    /// Returns the first text of this content, or an empty string.
    pub fn text(&self) -> &str {
        self.as_text().unwrap_or_default()
    }
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Content::Text(value)
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Content::Text(value.to_string())
    }
}

/// A single typed part of a multimodal message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentPart {
    /// A text part.
    Text {
        /// The text.
        text: String,
    },
    /// An image part, referenced by URL.
    ImageUrl {
        /// The image URL (or a base64 `data:` URI).
        url: String,
    },
}

/// A tool invocation requested by the model.
///
/// `arguments` is the JSON-encoded argument object, matching what OpenAI sends
/// and what Anthropic's structured `input` serializes to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// The id of the tool call, referenced by the matching tool message.
    pub id: String,
    /// The name of the tool to call.
    pub name: String,
    /// The arguments, as a JSON-encoded string.
    pub arguments: String,
}

/// Accumulates streamed tool-call fragments into a complete tool call.
///
/// Shared by the providers' SSE parsers.
#[derive(Debug, Default)]
pub(crate) struct PartialToolCall {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) arguments: String,
}

impl PartialToolCall {
    /// Converts the accumulated fragments into a complete [`ToolCall`].
    pub(crate) fn into_tool_call(self) -> ToolCall {
        ToolCall {
            id: self.id,
            name: self.name,
            arguments: self.arguments,
        }
    }
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: Role,
    /// The message content.
    pub content: Content,
    /// An optional participant name.
    pub name: Option<String>,
    /// The tool calls requested by the model (assistant messages).
    pub tool_calls: Vec<ToolCall>,
    /// The id of the tool call this message responds to (tool messages).
    pub tool_call_id: Option<String>,

    /// The reasoning (thinking) content produced by the model.
    ///
    /// Assistant messages only, and only when the request enabled reasoning.
    /// Providers that require it (Anthropic) round-trip this back with the
    /// message; OpenAI-compatible providers require it to be echoed alongside
    /// tool calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,

    /// The provider's opaque signature for [`Message::reasoning`], if any.
    ///
    /// Anthropic signs thinking blocks and rejects a turn that replays them
    /// without the matching signature; OpenAI-compatible providers leave this
    /// unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_signature: Option<String>,

    /// The persistence id of this message, once it has been stored.
    ///
    /// `None` for messages that have not been persisted yet. Set by
    /// `from_message` and used to target `set_context` / compaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The run that produced this message (`RunId`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,

    /// Whether this message is fed to the model as context.
    ///
    /// `true` (the default). Compaction sets it to `false` for messages
    /// replaced by a summary.
    #[serde(default = "default_true")]
    pub is_context: bool,

    /// The creation timestamp (unix seconds), assigned when persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

impl Message {
    /// Constructs a message from a role and content.
    pub fn new(role: Role, content: impl Into<Content>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
            reasoning: None,
            reasoning_signature: None,
            id: None,
            run_id: None,
            is_context: true,
            created_at: None,
        }
    }

    /// Constructs a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content.into())
    }

    /// Constructs a user message.
    pub fn user(content: impl Into<Content>) -> Self {
        Self::new(Role::User, content)
    }

    /// Constructs an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content.into())
    }

    /// Constructs a tool message carrying the result of a tool call.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        let mut message = Self::new(Role::Tool, content.into());
        message.tool_call_id = Some(tool_call_id.into());
        message
    }

    /// Sets the participant name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the tool calls produced by the model.
    pub fn tool_calls(mut self, tool_calls: impl IntoIterator<Item = ToolCall>) -> Self {
        self.tool_calls = tool_calls.into_iter().collect();
        self
    }

    /// Sets the id of the tool call this message responds to.
    pub fn tool_call_id(mut self, tool_call_id: impl Into<String>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }

    /// Sets the reasoning (thinking) content of an assistant message.
    pub fn reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }

    /// Returns the reasoning content of this message, if any.
    pub fn reasoning_text(&self) -> Option<&str> {
        self.reasoning.as_deref()
    }

    /// Sets the provider's opaque signature for the reasoning content.
    pub fn reasoning_signature(mut self, signature: impl Into<String>) -> Self {
        self.reasoning_signature = Some(signature.into());
        self
    }

    /// Sets the persistence id (used when re-hydrating from storage).
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the run id (used when attributing messages to a run).
    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    /// Returns the first text of this message, or an empty string.
    pub fn text(&self) -> &str {
        self.content.text()
    }
}

/// How the model should use the provided tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolChoice {
    /// The model will not call any tool.
    None,
    /// The model chooses between a message and tool calls.
    Auto,
    /// The model must call one or more tools.
    Required,
    /// The model must call the named tool.
    Tool(String),
}

/// The format the model's output must conform to.
///
/// This is the provider-neutral form of OpenAI's `response_format`. Providers
/// with no equivalent (Anthropic) ignore the field.
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseFormat {
    /// Plain text output (the default).
    Text,
    /// A single JSON object.
    JsonObject,
    /// A single JSON object matching the supplied JSON Schema.
    JsonSchema {
        /// The schema name shown to the model.
        name: String,
        /// The JSON Schema the output must satisfy.
        schema: Value,
        /// Whether the provider must enforce adherence strictly.
        strict: bool,
    },
}

/// How much reasoning (thinking) the model should produce before answering.
///
/// Reasoning-capable models think before they answer. Providers expose the
/// switch differently. DeepSeek takes a `thinking: {type}` object plus a
/// `reasoning_effort` level. Anthropic maps [`Reasoning::Enabled`] and
/// [`Reasoning::Effort`] to adaptive thinking, while [`Reasoning::Budget`] uses
/// legacy manual extended thinking on models that still support it. Providers
/// without a reasoning switch ignore the setting.
///
/// Leaving [`ChatRequest::reasoning`] unset sends nothing at all, so the
/// provider default applies (DeepSeek thinks by default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Reasoning {
    /// Do not think before answering.
    #[default]
    Disabled,
    /// Think with the provider's default intensity.
    ///
    /// Anthropic maps this to adaptive thinking without overriding the model's
    /// default effort.
    Enabled,
    /// Think with a fixed effort level.
    Effort(ReasoningEffort),
    /// Think within a token budget of at most this many tokens.
    ///
    /// Anthropic uses legacy manual extended thinking for this variant, which
    /// is unavailable on Claude 4.7 and later models. Providers without a
    /// budget concept fall back to [`Reasoning::Effort`].
    Budget(u32),
}

/// Per-token reasoning effort levels, ordered from least to most effort.
///
/// DeepSeek documents the scale as `low` / `high` / `max` (with `minimal` folded
/// into `low`, and legacy `medium` / `xhigh` folded into `high`). This ladder is
/// the documented one; [`ReasoningEffort::Medium`] is sent to providers that
/// accept it and folded into `high` for DeepSeek.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasoningEffort {
    /// Lowest thinking effort (DeepSeek's `low`).
    Low,
    /// The level between low and high, for providers that name one.
    Medium,
    /// High thinking effort, the default when thinking is on.
    High,
    /// Maximum thinking effort.
    Max,
}

/// The requested inference speed.
///
/// Fast mode is independent of [`Reasoning`]: callers may enable both on the
/// same request. Providers that do not explicitly support a speed are expected
/// to ignore this setting rather than approximate it with reasoning controls.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceSpeed {
    /// Use the provider's fast inference mode.
    Fast,
    /// A provider-specific speed value returned by an API.
    ///
    /// This variant preserves response values that do not have a neutral
    /// representation. It is not sent in requests by built-in providers.
    Other(String),
}

impl InferenceSpeed {
    pub(crate) fn from_provider(value: String) -> Self {
        if value == "fast" {
            Self::Fast
        } else {
            Self::Other(value)
        }
    }
}

impl Reasoning {
    /// Returns the effort level this setting asks for, if it maps to one.
    ///
    /// Budgets are mapped onto a level so that providers with discrete effort
    /// settings can still honor a budget: up to 2K tokens is [`ReasoningEffort::Low`],
    /// up to 8K is [`ReasoningEffort::Medium`], and anything above that is
    /// [`ReasoningEffort::High`]. Providers with a native budget concept
    /// (Anthropic's `thinking.budget_tokens`) use the budget directly.
    pub fn effort(&self) -> Option<ReasoningEffort> {
        match self {
            Self::Disabled => None,
            Self::Enabled => Some(ReasoningEffort::High),
            Self::Effort(effort) => Some(*effort),
            Self::Budget(tokens) => Some(match tokens {
                0..=2048 => ReasoningEffort::Low,
                2049..=8192 => ReasoningEffort::Medium,
                _ => ReasoningEffort::High,
            }),
        }
    }

    /// Returns whether reasoning is switched on.
    pub fn is_enabled(&self) -> bool {
        *self != Self::Disabled
    }
}

/// A chat completion request.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    /// The model name.
    pub model: String,
    /// The message history.
    pub messages: Vec<Message>,
    /// The tools the model may call.
    pub tools: Vec<ToolSpec>,
    /// Controls which tool the model may call.
    pub tool_choice: Option<ToolChoice>,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Nucleus sampling.
    pub top_p: Option<f32>,
    /// An upper bound on the number of tokens to generate.
    pub max_tokens: Option<u32>,
    /// Stop sequences.
    pub stop: Option<Vec<String>>,
    /// The required format of the model's output.
    pub response_format: Option<ResponseFormat>,
    /// How much the model should reason before answering.
    ///
    /// `None` leaves the provider default untouched.
    pub reasoning: Option<Reasoning>,
    /// The requested inference speed.
    ///
    /// `None` sends no speed field. Unsupported providers also omit the field.
    pub inference_speed: Option<InferenceSpeed>,
}

impl ChatRequest {
    /// Creates a new request for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            stop: None,
            response_format: None,
            reasoning: None,
            inference_speed: None,
        }
    }

    /// Appends a message.
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Appends multiple messages.
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Sets the tools the model may call.
    pub fn tools(mut self, tools: impl IntoIterator<Item = ToolSpec>) -> Self {
        self.tools = tools.into_iter().collect();
        self
    }

    /// Sets the tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Sets the sampling temperature.
    pub fn temperature(mut self, value: f32) -> Self {
        self.temperature = Some(value);
        self
    }

    /// Sets the nucleus sampling probability.
    pub fn top_p(mut self, value: f32) -> Self {
        self.top_p = Some(value);
        self
    }

    /// Sets the maximum number of tokens to generate.
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    /// Sets the stop sequences.
    pub fn stop(mut self, sequences: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.stop = Some(sequences.into_iter().map(Into::into).collect());
        self
    }

    /// Sets the required format of the model's output.
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Sets how much the model should reason before answering.
    ///
    /// ```
    /// use golden_agent::{ChatRequest, Reasoning};
    ///
    /// // DeepSeek-style on/off switch, and an OpenAI-style effort level.
    /// let on = ChatRequest::new("model").reasoning(Reasoning::Enabled);
    /// let deep = ChatRequest::new("model").reasoning(Reasoning::Budget(16_000));
    ///
    /// assert!(on.reasoning.is_some_and(|r| r.is_enabled()));
    /// assert_eq!(deep.reasoning, Some(Reasoning::Budget(16_000)));
    /// ```
    pub fn reasoning(mut self, reasoning: Reasoning) -> Self {
        self.reasoning = Some(reasoning);
        self
    }

    /// Sets the inference speed independently of reasoning/thinking.
    ///
    /// Built-in providers map [`InferenceSpeed::Fast`] to OpenAI's
    /// `service_tier: "fast"` or Anthropic's `speed: "fast"`. Providers with
    /// no explicit Fast Mode support omit the setting.
    ///
    /// ```
    /// use golden_agent::{ChatRequest, InferenceSpeed, Reasoning};
    ///
    /// let request = ChatRequest::new("model")
    ///     .inference_speed(InferenceSpeed::Fast)
    ///     .reasoning(Reasoning::Enabled);
    ///
    /// assert_eq!(request.inference_speed, Some(InferenceSpeed::Fast));
    /// assert_eq!(request.reasoning, Some(Reasoning::Enabled));
    /// ```
    pub fn inference_speed(mut self, speed: InferenceSpeed) -> Self {
        self.inference_speed = Some(speed);
        self
    }
}

/// A chat completion response.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatResponse {
    /// The unique response id.
    pub id: String,
    /// The model that produced the response.
    pub model: String,
    /// The assistant message.
    pub message: Message,
    /// Why generation stopped.
    pub finish_reason: Option<FinishReason>,
    /// Token usage statistics.
    pub usage: Option<Usage>,
    /// The actual provider-reported inference speed or service tier.
    ///
    /// This is populated for non-streaming responses when the provider returns
    /// the information. Unknown values are preserved in [`InferenceSpeed::Other`].
    pub inference_speed: Option<InferenceSpeed>,
}

impl ChatResponse {
    /// Returns the assistant's text, or an empty string.
    pub fn text(&self) -> &str {
        self.message.text()
    }

    /// Returns the model's reasoning content, or an empty string.
    ///
    /// Empty when reasoning was not enabled or the provider did not return it.
    pub fn reasoning(&self) -> &str {
        self.message.reasoning.as_deref().unwrap_or("")
    }

    /// Deserializes the assistant text into `T`.
    ///
    /// An optional ```` ```json ```` fence is stripped before parsing. On
    /// failure the returned [`Error::InvalidStructuredOutput`] carries the raw
    /// response text, so the caller can log it or feed it back for a retry.
    ///
    /// ```
    /// use golden_agent::{ChatResponse, Message, Role};
    ///
    /// let response = ChatResponse {
    ///     id: "id".into(),
    ///     model: "model".into(),
    ///     message: Message::assistant(r#"{"temperature_c": 28.5}"#),
    ///     finish_reason: None,
    ///     usage: None,
    ///     inference_speed: None,
    /// };
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Weather {
    ///     temperature_c: f64,
    /// }
    ///
    /// let weather: Weather = response.structured()?;
    /// assert_eq!(weather.temperature_c, 28.5);
    /// # Ok::<(), golden_agent::Error>(())
    /// ```
    pub fn structured<T: DeserializeOwned>(&self) -> Result<T, Error> {
        let text = self.text();
        serde_json::from_str(strip_json_fence(text)).map_err(|source| {
            Error::InvalidStructuredOutput {
                source,
                raw: text.to_string(),
            }
        })
    }
}

/// Strips an optional ```` ```json ```` fence and surrounding whitespace.
fn strip_json_fence(text: &str) -> &str {
    let text = text.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .unwrap_or(text);
    text.strip_suffix("```").unwrap_or(text).trim()
}

/// Why the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    /// The model reached a natural stopping point.
    Stop,
    /// The token limit was reached.
    Length,
    /// The model called one or more tools.
    ToolCalls,
    /// The output was filtered.
    ContentFilter,
    /// Any other reason, as reported by the provider.
    Other(String),
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usage {
    /// The number of tokens in the prompt.
    pub prompt_tokens: u32,
    /// The number of tokens in the generated completion.
    pub completion_tokens: u32,
    /// The total number of tokens used.
    pub total_tokens: u32,
    /// The number of completion tokens spent on reasoning.
    pub reasoning_tokens: Option<u32>,
}

fn default_true() -> bool {
    true
}
