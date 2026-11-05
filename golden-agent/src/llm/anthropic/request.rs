use serde::Serialize;
use serde_json::Value;

use super::tool::{Tool, ToolChoice};

/// A Messages API request body.
///
/// See <https://platform.claude.com/docs/en/api/messages>.
#[derive(Debug, Clone, Serialize)]
pub struct MessagesRequest {
    /// The model id.
    pub model: String,
    /// The maximum number of tokens to generate. Required by the Messages API.
    pub max_tokens: u32,
    /// The conversation turns.
    pub messages: Vec<InputMessage>,
    /// The system prompt. The Messages API has no `system` role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Definitions of tools the model may use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// How the model should use the provided tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Sampling temperature (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Custom text sequences that stop generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Whether to stream the response with server-sent events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// The inference speed to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<Speed>,
    /// Extended thinking switch. When enabled, the model thinks before it
    /// answers and returns `thinking` content blocks.
    ///
    /// DeepSeek accepts `enabled` / `disabled` here and ignores the
    /// `budget_tokens` that Anthropic uses; the thinking intensity comes from
    /// [`Self::output_config`] instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
    /// Output tuning, including the thinking effort level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
}

/// An Anthropic Messages API inference speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Speed {
    /// Anthropic Fast Mode.
    Fast,
}

/// The `thinking` object of a Messages API request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Thinking {
    /// Extended thinking, with a budget of `budget_tokens` tokens.
    ///
    /// Anthropic uses the budget to size its thinking; providers that expose a
    /// discrete effort scale (DeepSeek) ignore it.
    Enabled {
        /// The maximum number of tokens Claude may spend thinking.
        budget_tokens: u32,
    },
    /// Thinking is switched off.
    Disabled,
}

/// The `output_config` object of a Messages API request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OutputConfig {
    /// How hard the model should think before answering.
    ///
    /// Only `effort` is supported; DeepSeek reads `low` / `high` / `max`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
}

/// The thinking effort levels `output_config.effort` accepts.
///
/// DeepSeek documents `low` / `high` / `max`; the neutral `medium` level is
/// folded into `high` before it reaches the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    /// Low thinking effort.
    Low,
    /// High thinking effort (the provider default).
    High,
    /// Maximum thinking effort.
    Max,
}

/// A single input message.
#[derive(Debug, Clone, Serialize)]
pub struct InputMessage {
    /// The role of the message author.
    pub role: Role,
    /// The message content, as a list of content blocks.
    pub content: Vec<ContentBlock>,
}

/// The role of an input message. The Messages API only has `user` and
/// `assistant`; the system prompt is a top-level field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A user turn.
    User,
    /// An assistant turn.
    Assistant,
}

/// A content block in a request message.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text.
    Text {
        /// The text.
        text: String,
    },
    /// An image, referenced by URL.
    Image {
        /// The image source.
        source: ImageSource,
    },
    /// A tool invocation produced by the model, echoed back on an assistant turn.
    ToolUse {
        /// The id of the tool use, referenced by the matching `tool_result`.
        id: String,
        /// The tool name.
        name: String,
        /// The tool input, as produced by the model.
        input: Value,
    },
    /// A thinking block produced by the model, echoed back on an assistant turn.
    Thinking {
        /// The thinking text.
        thinking: String,
        /// The opaque signature that must be returned unchanged.
        signature: String,
    },
    /// The result of a tool invocation, sent on a user turn.
    ToolResult {
        /// The id of the `tool_use` block this result answers.
        tool_use_id: String,
        /// The tool result, as a string.
        content: String,
    },
}

/// The source of an image content block.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// An image referenced by URL.
    Url {
        /// The image URL.
        url: String,
    },
}
