use async_trait::async_trait;
use serde::Serialize;

use crate::error::Error;
use crate::llm::openai::{ChatRequest, ChatResponse};
use crate::llm::{ChatModel, ChatStream};
use crate::tool::{FromToolSpec, ToolSpec};

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

    /// Renders every registered `#[tool]` into Anthropic's wire format.
    pub fn tools() -> Vec<Tool> {
        crate::render_tools()
    }
}

/// Anthropic's wire format for a tool, sent in the Messages API `tools` field.
///
/// Anthropic names the JSON Schema `input_schema`, whereas OpenAI names it
/// `parameters`; [`FromToolSpec`] absorbs exactly this difference so the same
/// registered tools can serve either provider.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tool {
    /// The name the model uses to request the tool.
    pub name: String,
    /// A human-readable description that helps the model choose the tool.
    pub description: String,
    /// The JSON Schema describing the tool's parameters.
    pub input_schema: serde_json::Value,
}

impl FromToolSpec for Tool {
    fn from_tool_spec(spec: &ToolSpec) -> Self {
        Self {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            input_schema: spec.input_schema.clone(),
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
