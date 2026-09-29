use serde::Serialize;

use crate::tool::{FromToolSpec, ToolSpec};

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

/// How the model should use the provided tools.
///
/// Anthropic's `tool_choice` uses `auto`, `any`, `tool` and `none`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// The model decides whether to call a tool.
    Auto,
    /// The model must call one or more tools.
    Any,
    /// The model must not call any tool.
    None,
    /// The model must call the named tool.
    Tool {
        /// The name of the tool to call.
        name: String,
    },
}
