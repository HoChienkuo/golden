use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::Error;
use crate::llm::chat::{ChatModel, ChatRequest, ChatResponse, ToolCall};

use super::state::AgentState;
use super::tools::ToolSet;

/// The continuation a [`Middleware::wrap_model_call`] implementation invokes to
/// reach the next layer, ending at the chat model.
#[async_trait]
pub trait ModelHandler: Send + Sync {
    /// Calls the next layer with the given request.
    async fn handle(&self, request: ChatRequest) -> Result<ChatResponse, Error>;
}

/// The continuation a [`Middleware::wrap_tool_call`] implementation invokes to
/// reach the next layer, ending at the tool.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Calls the next layer with the given tool call.
    async fn handle(&self, call: ToolCall) -> Result<String, Error>;
}

/// Hooks that run around an agent run and around each model and tool call.
///
/// Every method has a default implementation, so a middleware overrides only
/// what it needs. Node-style hooks (`before_*` / `after_*`) run in order:
/// `before_*` first-to-last, `after_*` last-to-first. Wrap-style hooks nest like
/// function calls, so the first `wrap_*` middleware wraps all the others.
///
/// A middleware can implement dynamic behavior, for example retrying a model
/// call:
///
/// ```ignore
/// use async_trait::async_trait;
/// use golden_agent::{ChatRequest, ChatResponse, Error, Middleware, ModelHandler};
///
/// /// Retries a failed model call a few times.
/// struct RetryModel;
///
/// #[async_trait]
/// impl Middleware for RetryModel {
///     async fn wrap_model_call(
///         &self,
///         request: ChatRequest,
///         next: &dyn ModelHandler,
///     ) -> Result<ChatResponse, Error> {
///         let mut last = None;
///         for _ in 0..3 {
///             match next.handle(request.clone()).await {
///                 Ok(response) => return Ok(response),
///                 Err(error) => last = Some(error),
///             }
///         }
///         Err(last.expect("the loop always records an error"))
///     }
/// }
/// ```
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Runs once before the run starts.
    async fn before_agent(&self, _state: &mut AgentState) -> Result<(), Error> {
        Ok(())
    }

    /// Runs before each model call.
    async fn before_model(&self, _state: &mut AgentState) -> Result<(), Error> {
        Ok(())
    }

    /// Runs after each model call, before any of its tool calls are executed.
    async fn after_model(&self, _state: &mut AgentState) -> Result<(), Error> {
        Ok(())
    }

    /// Runs once after the run finishes.
    async fn after_agent(&self, _state: &mut AgentState) -> Result<(), Error> {
        Ok(())
    }

    /// Wraps each model call.
    async fn wrap_model_call(
        &self,
        request: ChatRequest,
        next: &dyn ModelHandler,
    ) -> Result<ChatResponse, Error> {
        next.handle(request).await
    }

    /// Wraps each tool call.
    async fn wrap_tool_call(
        &self,
        call: ToolCall,
        next: &dyn ToolHandler,
    ) -> Result<String, Error> {
        next.handle(call).await
    }
}

/// The terminal model handler: calls the wrapped model.
struct ModelCall {
    model: Arc<dyn ChatModel>,
}

#[async_trait]
impl ModelHandler for ModelCall {
    async fn handle(&self, request: ChatRequest) -> Result<ChatResponse, Error> {
        self.model.chat(&request).await
    }
}

/// One layer of the model-call onion.
struct LayeredModel {
    middleware: Arc<dyn Middleware>,
    next: Arc<dyn ModelHandler>,
}

#[async_trait]
impl ModelHandler for LayeredModel {
    async fn handle(&self, request: ChatRequest) -> Result<ChatResponse, Error> {
        self.middleware
            .wrap_model_call(request, self.next.as_ref())
            .await
    }
}

/// Builds the model-call chain, with the first middleware outermost.
pub(crate) fn model_chain(
    model: Arc<dyn ChatModel>,
    middlewares: &[Arc<dyn Middleware>],
) -> Arc<dyn ModelHandler> {
    let mut handler: Arc<dyn ModelHandler> = Arc::new(ModelCall { model });
    for middleware in middlewares.iter().rev() {
        handler = Arc::new(LayeredModel {
            middleware: Arc::clone(middleware),
            next: handler,
        });
    }
    handler
}

/// The terminal tool handler: resolves and runs the tool.
struct ToolCallTerminal {
    tools: ToolSet,
}

#[async_trait]
impl ToolHandler for ToolCallTerminal {
    async fn handle(&self, call: ToolCall) -> Result<String, Error> {
        let tool = self
            .tools
            .get(&call.name)
            .ok_or_else(|| Error::UnknownTool {
                name: call.name.clone(),
            })?;

        let arguments = parse_arguments(&call)?;
        tool.call(arguments).await
    }
}

/// Parses a tool call's JSON argument string into a value.
fn parse_arguments(call: &ToolCall) -> Result<Value, Error> {
    if call.arguments.trim().is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }

    serde_json::from_str(&call.arguments).map_err(|error| Error::Tool {
        name: call.name.clone(),
        message: format!("invalid arguments: {error}"),
    })
}

/// One layer of the tool-call onion.
struct LayeredTool {
    middleware: Arc<dyn Middleware>,
    next: Arc<dyn ToolHandler>,
}

#[async_trait]
impl ToolHandler for LayeredTool {
    async fn handle(&self, call: ToolCall) -> Result<String, Error> {
        self.middleware
            .wrap_tool_call(call, self.next.as_ref())
            .await
    }
}

/// Builds the tool-call chain, with the first middleware outermost.
pub(crate) fn tool_chain(
    tools: ToolSet,
    middlewares: &[Arc<dyn Middleware>],
) -> Arc<dyn ToolHandler> {
    let mut handler: Arc<dyn ToolHandler> = Arc::new(ToolCallTerminal { tools });
    for middleware in middlewares.iter().rev() {
        handler = Arc::new(LayeredTool {
            middleware: Arc::clone(middleware),
            next: handler,
        });
    }
    handler
}
