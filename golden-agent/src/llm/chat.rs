use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;

use crate::error::Error;
use crate::tool::ToolSpec;

/// A stream of incremental text tokens produced by a streaming chat completion.
pub type ChatStream<'a> = Pin<Box<dyn Stream<Item = Result<String, Error>> + Send + 'a>>;

/// A unified abstraction over chat-capable language models.
///
/// The request and response types are provider-neutral; each provider translates
/// them into (and out of) its own wire format internally.
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// Performs a single, non-streaming chat completion.
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error>;

    /// Performs a streaming chat completion, yielding text deltas.
    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_>;
}

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    /// The id of the tool call, referenced by the matching tool message.
    pub id: String,
    /// The name of the tool to call.
    pub name: String,
    /// The arguments, as a JSON-encoded string.
    pub arguments: String,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, PartialEq)]
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
}

impl ChatResponse {
    /// Returns the assistant's text, or an empty string.
    pub fn text(&self) -> &str {
        self.message.text()
    }
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
}
