use async_trait::async_trait;
use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::error::Error;

/// A callable function exposed to a language model as a tool.
///
/// Tools are normally declared with the [`#[tool]`](macro@crate::tool)
/// attribute macro, which derives the name, description and JSON Schema from
/// the function signature and doc comment. They can also be implemented
/// manually for full control.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The name of the tool, used by the model to request it.
    fn name(&self) -> &str;

    /// A human-readable description that helps the model choose when to call
    /// this tool.
    fn description(&self) -> &str;

    /// The JSON Schema describing the tool's parameters.
    fn schema(&self) -> Value;

    /// Invokes the tool with the model-provided JSON arguments and returns the
    /// JSON-encoded result.
    async fn call(&self, args: Value) -> Result<String, Error>;
}

/// A statically registered tool, collected through the inventory crate by the
/// [`#[tool]`](macro@crate::tool) attribute macro.
///
/// This mirrors `golden-boot`'s `RouteDefinition` mechanism. The [`call`]
/// function deserializes the model-provided arguments, invokes the underlying
/// async function, and returns the serialized result wrapped in a boxed
/// future.
///
/// [`call`]: ToolDefinition::call
#[doc(hidden)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: fn() -> Value,
    pub call: fn(Value) -> BoxFuture<'static, Result<String, Error>>,
}

inventory::collect!(ToolDefinition);

/// Returns every statically registered tool, sorted by name.
#[doc(hidden)]
pub fn registered_tools() -> Vec<&'static ToolDefinition> {
    let mut tools = inventory::iter::<ToolDefinition>
        .into_iter()
        .collect::<Vec<_>>();
    tools.sort_by_key(|tool| tool.name);
    tools
}

/// Converts every statically registered tool into an OpenAI function tool,
/// ready to be attached to a [`ChatRequest`](crate::llm::openai::ChatRequest).
#[doc(hidden)]
pub fn into_chat_tools() -> Vec<crate::llm::openai::Tool> {
    registered_tools()
        .into_iter()
        .map(|tool| {
            crate::llm::openai::Tool::function(
                tool.name,
                Some(tool.description.to_string()),
                Some((tool.schema)()),
            )
        })
        .collect()
}
