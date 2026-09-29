//! A ReAct-style agent that drives a chat model and its tools in a loop.

mod context;
mod middleware;
mod prompt;
mod result;
mod retry;
mod state;
mod tools;

use std::pin::Pin;
use std::sync::Arc;

use futures_core::Stream;
use futures_util::StreamExt;

use crate::error::Error;
use crate::llm::chat::{ChatEvent, ChatModel, ChatRequest, Message, ToolCall};

pub use context::Context;
pub use middleware::{Middleware, ModelHandler, ToolHandler};
pub use prompt::DynamicPrompt;
pub use result::AgentResult;
pub use retry::Retry;
pub use state::AgentState;
pub use tools::ToolSet;

/// Runtime configuration for an [`Agent`].
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// The maximum number of model/tool cycles before the run fails.
    pub max_steps: usize,
    /// Whether a failing tool aborts the run instead of being fed back.
    ///
    /// When `false` (the default), a tool error becomes a tool message so the
    /// model can react to it.
    pub abort_on_tool_error: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            abort_on_tool_error: false,
        }
    }
}

/// An event from a streaming agent run.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    /// A chunk of assistant text.
    Text(String),
    /// A tool call the model requested.
    ToolCall(ToolCall),
    /// A tool finished; `content` holds its result.
    ToolResult {
        /// The tool call that was executed.
        call: ToolCall,
        /// The tool's result.
        content: String,
    },
    /// The run finished.
    Done,
}

/// A stream of [`AgentEvent`]s produced by [`Agent::stream`].
pub type AgentStream<'a> = Pin<Box<dyn Stream<Item = Result<AgentEvent, Error>> + Send + 'a>>;

/// A ReAct-style agent.
///
/// It calls the model, runs the tools the model requests, feeds the results
/// back, and repeats until the model stops calling tools or `max_steps` is
/// reached.
pub struct Agent {
    model: Arc<dyn ChatModel>,
    request: ChatRequest,
    tools: ToolSet,
    system_prompt: Option<String>,
    middlewares: Vec<Arc<dyn Middleware>>,
    config: AgentConfig,
}

impl Agent {
    /// Starts building an agent.
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Runs the agent on a single input message with an empty context.
    pub async fn invoke(&self, input: impl Into<Message>) -> Result<AgentResult, Error> {
        self.invoke_with_context(input, Context::new()).await
    }

    /// Runs the agent on a single input message with the given run context.
    pub async fn invoke_with_context(
        &self,
        input: impl Into<Message>,
        context: Context,
    ) -> Result<AgentResult, Error> {
        self.invoke_messages_with_context([input.into()], context)
            .await
    }

    /// Runs the agent on an existing conversation with an empty context.
    pub async fn invoke_messages(
        &self,
        messages: impl IntoIterator<Item = Message>,
    ) -> Result<AgentResult, Error> {
        self.invoke_messages_with_context(messages, Context::new())
            .await
    }

    /// Runs the agent on an existing conversation with the given run context.
    ///
    /// The [`system_prompt`](AgentBuilder::system_prompt), if any, is prepended,
    /// and `context` becomes [`AgentState::context`].
    pub async fn invoke_messages_with_context(
        &self,
        messages: impl IntoIterator<Item = Message>,
        context: Context,
    ) -> Result<AgentResult, Error> {
        let mut state = AgentState::new();
        if let Some(prompt) = &self.system_prompt {
            state.messages.push(Message::system(prompt.clone()));
        }
        state.messages.extend(messages);
        state.context = context;

        self.run(&mut state).await?;
        Ok(AgentResult::new(state))
    }

    /// Streams a run on a single input message with an empty context.
    pub fn stream(&self, input: impl Into<Message>) -> AgentStream<'_> {
        self.stream_with_context(input, Context::new())
    }

    /// Streams a run on a single input message with the given run context.
    pub fn stream_with_context(
        &self,
        input: impl Into<Message>,
        context: Context,
    ) -> AgentStream<'_> {
        self.stream_messages_with_context([input.into()], context)
    }

    /// Streams a run on an existing conversation with an empty context.
    pub fn stream_messages(&self, messages: impl IntoIterator<Item = Message>) -> AgentStream<'_> {
        self.stream_messages_with_context(messages, Context::new())
    }

    /// Streams a run on an existing conversation with the given run context.
    ///
    /// Node-style middleware hooks and [`Middleware::wrap_tool_call`] run as in
    /// [`invoke`](Agent::invoke). [`Middleware::wrap_model_call`] is skipped,
    /// because it operates on a complete response rather than a delta stream.
    pub fn stream_messages_with_context(
        &self,
        messages: impl IntoIterator<Item = Message>,
        context: Context,
    ) -> AgentStream<'_> {
        let input: Vec<Message> = messages.into_iter().collect();
        let system_prompt = self.system_prompt.clone();
        let middlewares = self.middlewares.clone();
        let tools = self.tools.clone();
        let request = self.request.clone();
        let config = self.config.clone();
        let model = Arc::clone(&self.model);

        Box::pin(async_stream::stream! {
            let mut state = AgentState::new();
            if let Some(prompt) = &system_prompt {
                state.messages.push(Message::system(prompt.clone()));
            }
            state.messages.extend(input);
            state.context = context;

            for middleware in &middlewares {
                if let Err(error) = middleware.before_agent(&mut state).await {
                    yield Err(error);
                    return;
                }
            }

            let tool = middleware::tool_chain(tools.clone(), &middlewares);

            let mut steps = 0;
            loop {
                if steps >= config.max_steps {
                    yield Err(Error::MaxStepsExceeded { steps: config.max_steps });
                    return;
                }

                for middleware in &middlewares {
                    if let Err(error) = middleware.before_model(&mut state).await {
                        yield Err(error);
                        return;
                    }
                }

                let mut step_request = request.clone();
                step_request.messages = state.messages.clone();
                step_request.tools = tools.specs();

                let mut text = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                let mut stream = model.chat_stream(&step_request);
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(ChatEvent::Text(delta)) => {
                            text.push_str(&delta);
                            yield Ok(AgentEvent::Text(delta));
                        }
                        Ok(ChatEvent::ToolCall(call)) => tool_calls.push(call),
                        Err(error) => {
                            yield Err(error);
                            return;
                        }
                    }
                }

                state
                    .messages
                    .push(Message::assistant(text).tool_calls(tool_calls.clone()));

                for middleware in &middlewares {
                    if let Err(error) = middleware.after_model(&mut state).await {
                        yield Err(error);
                        return;
                    }
                }

                if tool_calls.is_empty() {
                    for middleware in &middlewares {
                        if let Err(error) = middleware.after_agent(&mut state).await {
                            yield Err(error);
                            return;
                        }
                    }
                    yield Ok(AgentEvent::Done);
                    return;
                }

                for call in tool_calls {
                    yield Ok(AgentEvent::ToolCall(call.clone()));

                    let content = match tool.handle(call.clone()).await {
                        Ok(content) => content,
                        Err(error) if config.abort_on_tool_error => {
                            yield Err(error);
                            return;
                        }
                        Err(error) => format!("Error: {error}"),
                    };

                    state.messages.push(Message::tool(call.id.clone(), content.clone()));
                    yield Ok(AgentEvent::ToolResult {
                        call,
                        content,
                    });
                }

                steps += 1;
            }
        })
    }

    /// Returns the tools the agent may call.
    pub fn tools(&self) -> &ToolSet {
        &self.tools
    }

    async fn run(&self, state: &mut AgentState) -> Result<(), Error> {
        for middleware in &self.middlewares {
            middleware.before_agent(state).await?;
        }

        let model = middleware::model_chain(Arc::clone(&self.model), &self.middlewares);
        let tool = middleware::tool_chain(self.tools.clone(), &self.middlewares);

        let mut steps = 0;
        loop {
            if steps >= self.config.max_steps {
                return Err(Error::MaxStepsExceeded {
                    steps: self.config.max_steps,
                });
            }

            for middleware in &self.middlewares {
                middleware.before_model(state).await?;
            }

            let response = model.handle(self.build_request(state)).await?;
            state.messages.push(response.message.clone());

            for middleware in &self.middlewares {
                middleware.after_model(state).await?;
            }

            if response.message.tool_calls.is_empty() {
                break;
            }

            for call in &response.message.tool_calls {
                let content = match tool.handle(call.clone()).await {
                    Ok(content) => content,
                    Err(error) if self.config.abort_on_tool_error => return Err(error),
                    Err(error) => format!("Error: {error}"),
                };
                state.messages.push(Message::tool(call.id.clone(), content));
            }

            steps += 1;
        }

        for middleware in &self.middlewares {
            middleware.after_agent(state).await?;
        }

        Ok(())
    }

    /// Builds the request for one step from the base request.
    fn build_request(&self, state: &AgentState) -> ChatRequest {
        let mut request = self.request.clone();
        request.messages = state.messages.clone();
        request.tools = self.tools.specs();
        request
    }
}

/// Builds an [`Agent`].
#[derive(Default)]
pub struct AgentBuilder {
    model: Option<Arc<dyn ChatModel>>,
    request: Option<ChatRequest>,
    tools: ToolSet,
    system_prompt: Option<String>,
    middlewares: Vec<Arc<dyn Middleware>>,
    config: AgentConfig,
}

impl AgentBuilder {
    /// Sets the chat model from any [`ChatModel`] implementation.
    pub fn model(mut self, model: impl ChatModel + 'static) -> Self {
        self.model = Some(Arc::new(model));
        self
    }

    /// Sets the chat model from a shared handle.
    pub fn shared_model(mut self, model: Arc<dyn ChatModel>) -> Self {
        self.model = Some(model);
        self
    }

    /// Sets the base request.
    ///
    /// Its model name and sampling options are kept; its messages and tools are
    /// replaced on every step.
    pub fn request(mut self, request: ChatRequest) -> Self {
        self.request = Some(request);
        self
    }

    /// Sets the tools the agent may call.
    pub fn tools(mut self, tools: ToolSet) -> Self {
        self.tools = tools;
        self
    }

    /// Prepends a system message to every run.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Appends a middleware.
    pub fn middleware(mut self, middleware: impl Middleware + 'static) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    /// Replaces the runtime configuration.
    pub fn config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the maximum number of model/tool cycles.
    pub fn max_steps(mut self, max_steps: usize) -> Self {
        self.config.max_steps = max_steps;
        self
    }

    /// Builds the agent.
    ///
    /// Fails with [`Error::MissingSetting`] if the model or base request was not
    /// provided.
    pub fn build(self) -> Result<Agent, Error> {
        let model = self.model.ok_or(Error::MissingSetting { name: "model" })?;
        let request = self
            .request
            .ok_or(Error::MissingSetting { name: "request" })?;

        Ok(Agent {
            model,
            request,
            tools: self.tools,
            system_prompt: self.system_prompt,
            middlewares: self.middlewares,
            config: self.config,
        })
    }
}
