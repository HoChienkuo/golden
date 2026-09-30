use std::borrow::Cow;

use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::error::Error;

/// A provider-neutral description of a tool exposed to a language model.
///
/// This is the common currency between the `#[tool]` registry and each
/// provider's wire format. A provider converts it into its own tool type by
/// implementing [`FromToolSpec`], which is how the same registered tools can be
/// rendered for OpenAI, Anthropic or any other backend.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolSpec {
    /// The name the model uses to request the tool.
    pub name: Cow<'static, str>,
    /// A human-readable description that helps the model choose the tool.
    pub description: Cow<'static, str>,
    /// The JSON Schema describing the tool's parameters.
    pub input_schema: Value,
}

impl ToolSpec {
    /// Builds a spec from a name, a description and its JSON Schema.
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        description: impl Into<Cow<'static, str>>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// A callable tool exposed to a language model.
///
/// Tools are normally declared with the [`#[tool]`](macro@crate::tool) attribute,
/// which registers a [`ToolDefinition`] implementing this trait. Implement it
/// directly for full control, such as carrying shared state.
pub trait Tool: Send + Sync {
    /// The provider-neutral description of this tool.
    fn spec(&self) -> ToolSpec;

    /// Invokes the tool with the model-provided JSON arguments and returns the
    /// JSON-encoded result.
    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>>;
}

impl<T: Tool + ?Sized> Tool for &T {
    fn spec(&self) -> ToolSpec {
        (**self).spec()
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        (**self).call(args)
    }
}

/// Describes a Rust type as a JSON Schema object for tool parameters.
///
/// [`#[tool]`](macro@crate::tool) maps primitive parameter types
/// (`String`, integers, floats, `bool`, `Vec<T>`, `Option<T>`) straight to their
/// schema. Any other parameter type — a `struct` carrying the model's expected
/// arguments, for example — must implement this trait so the model can see the
/// type's fields instead of a bare `{"type": "object"}`.
///
/// Derive it rather than writing the implementation by hand:
///
/// ```ignore
/// use golden_agent::ToolSchema;
///
/// #[derive(serde::Deserialize, ToolSchema)]
/// struct Order {
///     /// Name of the item to order.
///     item: String,
///     /// How many units to order.
///     quantity: u32,
/// }
///
/// /// Place an order for a structured item.
/// #[golden_agent::tool]
/// async fn place_order(order: Order) -> String {
///     format!("Ordered {} x {}", order.quantity, order.item)
/// }
/// ```
///
/// `#[param(description = "...")]` and `#[param(required = false)]` are read
/// from each field the same way they are read from a `#[tool]` function
/// parameter; a field without `#[param(description ...)]` falls back to its
/// `///` doc comment, `Option<T>` fields are optional unless overridden. The
/// derive requires a concrete struct — generic parameters are rejected with a
/// compile error, because the generated impl cannot forward them or infer the
/// `T: ToolSchema` bounds their fields would need.
pub trait ToolSchema {
    /// Builds this type's JSON Schema as a `serde_json::Value`.
    fn schema() -> Value;
}

/// A provider's wire-format tool, built from a neutral [`ToolSpec`].
///
/// Every provider implements this for its own tool type — for example
/// [`openai::Tool`](crate::llm::openai::Tool) and
/// [`anthropic::Tool`](crate::llm::anthropic::Tool) — so that a single set of
/// registered tools can be rendered into any provider's request.
pub trait FromToolSpec: Sized {
    /// Builds the provider representation from a neutral spec.
    fn from_tool_spec(spec: &ToolSpec) -> Self;
}

/// A statically registered tool, collected through the `inventory` crate by the
/// [`#[tool]`](macro@crate::tool) attribute macro.
///
/// The function-pointer fields let the macro register a plain `async fn` without
/// instantiating a value; prefer the [`Tool`] trait when invoking it.
#[doc(hidden)]
pub struct ToolDefinition {
    /// The tool name, used by the model to request it.
    pub name: &'static str,
    /// A human-readable description of the tool.
    pub description: &'static str,
    /// Builds the JSON Schema describing the tool's parameters.
    pub schema: fn() -> Value,
    /// Deserializes the arguments, invokes the function and serializes the result.
    pub call: fn(Value) -> BoxFuture<'static, Result<String, Error>>,
}

impl Tool for ToolDefinition {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: Cow::Borrowed(self.name),
            description: Cow::Borrowed(self.description),
            input_schema: (self.schema)(),
        }
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        (self.call)(args)
    }
}

inventory::collect!(ToolDefinition);

/// Returns every statically registered tool, sorted by name.
///
/// Fails with [`Error::DuplicateTool`] if two registered tools share a name, so
/// a collision surfaces at startup instead of silently dropping a tool.
pub fn registered_tools() -> Result<Vec<&'static ToolDefinition>, Error> {
    let mut tools = inventory::iter::<ToolDefinition>
        .into_iter()
        .collect::<Vec<_>>();
    tools.sort_by_key(|tool| tool.name);

    // Duplicates are adjacent after sorting.
    for pair in tools.windows(2) {
        if pair[0].name == pair[1].name {
            return Err(Error::DuplicateTool {
                name: pair[0].name.to_string(),
            });
        }
    }

    Ok(tools)
}

/// Returns the neutral [`ToolSpec`] of every registered tool, sorted by name.
///
/// Fails with [`Error::DuplicateTool`] if two registered tools share a name.
pub fn tool_specs() -> Result<Vec<ToolSpec>, Error> {
    Ok(registered_tools()?
        .into_iter()
        .map(|tool| tool.spec())
        .collect())
}

/// Renders every registered tool into a provider's wire format.
///
/// The provider type is inferred from the binding, so the same call renders the
/// tools for any provider implementing [`FromToolSpec`]:
///
/// ```
/// use golden_agent::llm::{anthropic, openai};
///
/// let openai_tools: Vec<openai::Tool> = golden_agent::render_tools()?;
/// let anthropic_tools: Vec<anthropic::Tool> = golden_agent::render_tools()?;
/// # Ok::<(), golden_agent::Error>(())
/// ```
///
/// Fails with [`Error::DuplicateTool`] if two registered tools share a name.
pub fn render_tools<T: FromToolSpec>() -> Result<Vec<T>, Error> {
    Ok(tool_specs()?.iter().map(T::from_tool_spec).collect())
}
