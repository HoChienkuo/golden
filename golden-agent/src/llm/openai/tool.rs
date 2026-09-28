use serde::{Deserialize, Serialize};

/// The type discriminator used by tool-related objects.
///
/// OpenAI currently defines only `"function"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Function,
}

/// A tool the model may call, sent in [`crate::llm::openai::ChatRequest::tools`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// The tool type, always `function`.
    pub r#type: ToolType,
    /// The function definition.
    pub function: FunctionDefinition,
}

impl Tool {
    /// Constructs a function tool from its name, description and JSON Schema
    /// parameters.
    pub fn function(
        name: impl Into<String>,
        description: Option<String>,
        parameters: Option<serde_json::Value>,
    ) -> Self {
        Self {
            r#type: ToolType::Function,
            function: FunctionDefinition {
                name: name.into(),
                description,
                parameters,
                strict: None,
            },
        }
    }
}

/// The definition of a callable function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// The name of the function to call.
    pub name: String,
    /// A description of what the function does, used by the model to choose
    /// when and how to call it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The parameters the function accepts, described as a JSON Schema object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Whether to enable strict schema adherence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// A tool call emitted by the model, or echoed back in an assistant message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// The id of the tool call, referenced by the matching `tool` message.
    pub id: String,
    /// The tool type, always `function`.
    pub r#type: ToolType,
    /// The function the model called.
    pub function: FunctionCall,
}

/// The function name and arguments produced by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to call the function with, as a JSON-encoded string.
    pub arguments: String,
}

/// Controls which (if any) tool the model may call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// A coarse selection mode: `none`, `auto` or `required`.
    Mode(ToolChoiceMode),
    /// Forces a particular function to be called.
    Named(NamedToolChoice),
}

/// A coarse tool selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceMode {
    /// The model will not call any tool.
    None,
    /// The model chooses between message and tool calls.
    Auto,
    /// The model must call one or more tools.
    Required,
}

/// Forces a specific function to be called.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedToolChoice {
    /// The tool type, always `function`.
    pub r#type: ToolType,
    /// The function to call.
    pub function: NamedFunction,
}

/// The name of the function to force.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedFunction {
    /// The name of the function to call.
    pub name: String,
}

impl From<ToolChoiceMode> for ToolChoice {
    fn from(value: ToolChoiceMode) -> Self {
        ToolChoice::Mode(value)
    }
}

impl From<NamedToolChoice> for ToolChoice {
    fn from(value: NamedToolChoice) -> Self {
        ToolChoice::Named(value)
    }
}
