#![recursion_limit = "256"]

mod error;
pub mod llm;
pub mod tool;

pub use error::{ApiErrorBody, ApiErrorDetail, Error};
pub use llm::chat::{ChatModel, ChatStream};
pub use llm::openai::*;
pub use llm::{AnthropicLlm, AnthropicModel, DeepSeekLlm, DeepSeekModel};
pub use tool::{
    FromToolSpec, Tool, ToolDefinition, ToolSpec, registered_tools, render_tools, tool_specs,
};

#[doc(inline)]
pub use golden_macros::tool;

#[doc(hidden)]
pub mod __private {
    pub use crate::tool::ToolDefinition;
    pub use futures_util;
    pub use inventory;
    pub use serde_json;
}
