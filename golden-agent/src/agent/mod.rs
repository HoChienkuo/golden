//! A ReAct-style agent that drives a chat model and its tools in a loop.

mod context;
mod metadata;
mod middleware;
mod result;
mod state;
mod tools;

#[cfg(test)]
mod tests;

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use futures_core::Stream;
use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;

use crate::checkpointer::CheckpointMessage;
use crate::checkpointer::Checkpointer;
use crate::error::Error;
use crate::llm::chat::{
    ChatEvent, ChatModel, ChatRequest, Content, ContentPart, Message, Role, ToolCall, Usage,
};
use crate::store::Store;
use crate::store::{RecallTool, RememberTool};

pub use context::Context;
pub use metadata::{RunId, ThreadId};
pub use middleware::{DynamicPrompt, Middleware, ModelHandler, Retry, ToolHandler};
pub use result::AgentResult;
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
    /// How many tool calls from one model response may run at the same time.
    ///
    /// `1` (the default) runs them sequentially; higher values execute
    /// independent calls concurrently while results still feed back to the
    /// model in the order the model requested them.
    pub max_concurrent_tool_calls: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            abort_on_tool_error: false,
            max_concurrent_tool_calls: 1,
        }
    }
}

/// An event from a streaming agent run.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    /// A chunk of the model's reasoning (thinking) output.
    ///
    /// Emitted only when the agent's request enables reasoning and the model
    /// streams it. Each step's thinking arrives before that step's text.
    Reasoning(String),
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
pub struct AgentStream<'a> {
    inner: Pin<Box<dyn Stream<Item = Result<StreamUpdate, Error>> + Send + 'a>>,
    usage: Option<Usage>,
}

impl AgentStream<'_> {
    /// Usage of the latest completed model call while polling this stream.
    ///
    /// Returns `None` before a call completes or if the latest completed call
    /// did not report usage.
    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }
}

impl Stream for AgentStream<'_> {
    type Item = Result<AgentEvent, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match std::task::ready!(this.inner.as_mut().poll_next(cx)) {
                Some(Ok(StreamUpdate::Usage(usage))) => this.usage = usage,
                Some(Ok(StreamUpdate::Event(event))) => return Poll::Ready(Some(Ok(event))),
                Some(Err(error)) => return Poll::Ready(Some(Err(error))),
                None => return Poll::Ready(None),
            }
        }
    }
}

enum StreamUpdate {
    Event(AgentEvent),
    Usage(Option<Usage>),
}

/// A ReAct-style agent.
///
/// It calls the model, runs the tools the model requests, feeds the results
/// back, and repeats until the model stops calling tools, a tool declared
/// `return_direct` supplies the final answer, or `max_steps` is reached.
pub struct Agent {
    model: Arc<dyn ChatModel>,
    request: ChatRequest,
    tools: ToolSet,
    system_prompt: Option<String>,
    middlewares: Vec<Arc<dyn Middleware>>,
    config: AgentConfig,
    checkpointer: Option<Arc<dyn Checkpointer>>,
    store: Option<Arc<dyn Store>>,
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
    /// The conversation starts with a system message. See
    /// [`AgentBuilder::system_prompt`] for how it is selected.
    /// The supplied `context` becomes [`AgentState::context`].
    pub async fn invoke_messages_with_context(
        &self,
        messages: impl IntoIterator<Item = Message>,
        context: Context,
    ) -> Result<AgentResult, Error> {
        let mut pending: Vec<Message> = messages.into_iter().collect();
        let mut state = self.start_run(context, &pending).await?;
        let usage = self.run(&mut state, &mut pending).await?;
        Ok(AgentResult::new(state, usage))
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
        let mut pending: Vec<Message> = messages.into_iter().collect();
        let middlewares = self.middlewares.clone();
        let tools = self.tools.clone();
        let request = self.request.clone();
        let config = self.config.clone();
        let model = Arc::clone(&self.model);

        let inner = Box::pin(async_stream::stream! {
            let mut state = match self.start_run(context, &pending).await {
                Ok(state) => state,
                Err(error) => {
                    yield Err(error);
                    return;
                }
            };

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
                let mut reasoning = String::new();
                let mut reasoning_signature = None;
                let mut completed_parts: Vec<ContentPart> = Vec::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();
                let mut usage = None;

                let mut stream = model.chat_stream(&step_request);
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(ChatEvent::Reasoning(delta)) => {
                            reasoning.push_str(&delta);
                            yield Ok(StreamUpdate::Event(AgentEvent::Reasoning(delta)));
                        }
                        Ok(ChatEvent::ReasoningSignature(signature)) => {
                            reasoning_signature = Some(signature);
                        }
                        Ok(ChatEvent::Text(delta)) => {
                            text.push_str(&delta);
                            yield Ok(StreamUpdate::Event(AgentEvent::Text(delta)));
                        }
                        Ok(ChatEvent::ContentPart(part)) => completed_parts.push(part),
                        Ok(ChatEvent::ToolCall(call)) => tool_calls.push(call),
                        Ok(ChatEvent::Usage(value)) => usage = Some(value),
                        Err(error) => {
                            yield Err(error);
                            return;
                        }
                    }
                }

                let message = if completed_parts.is_empty() {
                    let mut message = Message::assistant(text).tool_calls(tool_calls.clone());
                    if !reasoning.is_empty() {
                        message.reasoning = Some(reasoning);
                    }
                    message.reasoning_signature = reasoning_signature;
                    message
                } else {
                    let message = Message::parts(Role::Assistant, completed_parts);
                    tool_calls = message.tool_calls.clone();
                    message
                };
                record_message(&mut state, &mut pending, message);

                for middleware in &middlewares {
                    if let Err(error) = middleware.after_model(&mut state).await {
                        yield Err(error);
                        return;
                    }
                }

                yield Ok(StreamUpdate::Usage(usage));

                if tool_calls.is_empty() {
                    if let Err(error) = self.persist_step(&state, &mut pending).await {
                        yield Err(error);
                        return;
                    }
                    for middleware in &middlewares {
                        if let Err(error) = middleware.after_agent(&mut state).await {
                            yield Err(error);
                            return;
                        }
                    }
                    yield Ok(StreamUpdate::Event(AgentEvent::Done));
                    return;
                }

                // Announce every call the model requested before running any.
                for call in &tool_calls {
                    yield Ok(StreamUpdate::Event(AgentEvent::ToolCall(call.clone())));
                }

                // Run the batch concurrently, yielding each result live in
                // completion order. `results` is indexed by call position.
                let mut results: Vec<Option<(ToolCall, String, bool)>> =
                    (0..tool_calls.len()).map(|_| None).collect();
                let mut batch = Box::pin(execute_tool_batch(
                    Arc::clone(&tool),
                    tool_calls,
                    config.max_concurrent_tool_calls,
                ));

                while let Some(outcome) = batch.next().await {
                    let ToolOutcome { index, call, result } = outcome;
                    let (content, ok) = match result {
                        Ok(content) => (content, true),
                        Err(error) if config.abort_on_tool_error => {
                            yield Err(error);
                            return;
                        }
                        Err(error) => (format!("Error: {error}"), false),
                    };
                    results[index] = Some((call.clone(), content.clone(), ok));
                    yield Ok(StreamUpdate::Event(AgentEvent::ToolResult { call, content }));
                }


                // Feed results back in the model's call order, whatever order
                // the calls finished in. (flatten() preserves index order.)
                let mut direct: Option<String> = None;
                for (call, content, ok) in results.into_iter().flatten() {
                    if ok && direct.is_none() && returns_direct(&tools, &call.name) {
                        direct = Some(content.clone());
                    }
                    let message = if ok {
                        Message::tool(call.id.clone(), content)
                    } else {
                        Message::tool_error(call.id.clone(), content)
                    };
                    record_message(&mut state, &mut pending, message);
                }

                if let Err(error) = self.persist_step(&state, &mut pending).await {
                    yield Err(error);
                    return;
                }


                steps += 1;

                // A `return_direct` tool's result is the final answer: echo it
                // as the last assistant message and end the run without
                // another model call.
                if let Some(content) = direct {
                    record_message(
                        &mut state,
                        &mut pending,
                        Message::assistant(direct_answer(&content)),
                    );
                    if let Err(error) = self.persist_step(&state, &mut pending).await {
                        yield Err(error);
                        return;
                    }
                    for middleware in &middlewares {
                        if let Err(error) = middleware.after_agent(&mut state).await {
                            yield Err(error);
                            return;
                        }
                    }
                    yield Ok(StreamUpdate::Event(AgentEvent::Done));
                    return;
                }
            }
        });
        AgentStream { inner, usage: None }
    }

    /// Returns the tools the agent may call.
    pub fn tools(&self) -> &ToolSet {
        &self.tools
    }

    async fn start_run(
        &self,
        mut context: Context,
        input: &[Message],
    ) -> Result<AgentState, Error> {
        if context.get::<RunId>().is_none() {
            context.insert(RunId(generate_id()));
        }

        let mut messages = Vec::new();
        if let Some(cp) = &self.checkpointer
            && let Some(tid) = context.get::<ThreadId>()
        {
            messages.extend(cp.load(&tid.0).await?.into_iter().map(Message::from));
        }

        if let Some(prompt) = &self.system_prompt {
            apply_system_prompt(&mut messages, prompt);
        }
        messages.extend_from_slice(input);
        if !messages
            .first()
            .is_some_and(|message| message.role == Role::System)
        {
            messages.insert(0, Message::system(DEFAULT_SYSTEM_PROMPT));
        }

        Ok(AgentState { messages, context })
    }

    async fn run(
        &self,
        state: &mut AgentState,
        pending: &mut Vec<Message>,
    ) -> Result<Option<Usage>, Error> {
        for middleware in &self.middlewares {
            middleware.before_agent(state).await?;
        }

        let model = middleware::model_chain(Arc::clone(&self.model), &self.middlewares);
        let tool = middleware::tool_chain(self.tools.clone(), &self.middlewares);

        let mut steps = 0;
        let usage = loop {
            if steps >= self.config.max_steps {
                return Err(Error::MaxStepsExceeded {
                    steps: self.config.max_steps,
                });
            }

            for middleware in &self.middlewares {
                middleware.before_model(state).await?;
            }

            let response = model.handle(self.build_request(state)).await?;
            record_message(state, pending, response.message.clone());

            for middleware in &self.middlewares {
                middleware.after_model(state).await?;
            }

            if response.message.tool_calls.is_empty() {
                // persist if last
                self.persist_step(state, pending).await?;
                break response.usage;
            }

            let calls = response.message.tool_calls.clone();
            let mut outcomes: Vec<ToolOutcome> = execute_tool_batch(
                Arc::clone(&tool),
                calls.clone(),
                self.config.max_concurrent_tool_calls,
            )
            .collect()
            .await;
            outcomes.sort_by_key(|outcome| outcome.index);

            let mut direct: Option<String> = None;
            for (call, outcome) in calls.iter().zip(outcomes) {
                match outcome.result {
                    Ok(content) => {
                        if direct.is_none() && returns_direct(&self.tools, &call.name) {
                            direct = Some(content.clone());
                        }
                        record_message(state, pending, Message::tool(call.id.clone(), content));
                    }
                    Err(error) if self.config.abort_on_tool_error => return Err(error),
                    Err(error) => {
                        let message =
                            Message::tool_error(call.id.clone(), format!("Error: {error}"));
                        record_message(state, pending, message);
                    }
                }
            }

            // A `return_direct` tool's result is the final answer: echo it as
            // the last assistant message and end the run without another model
            // call.
            if let Some(content) = direct {
                record_message(state, pending, Message::assistant(direct_answer(&content)));
                self.persist_step(state, pending).await?;
                break response.usage;
            }

            self.persist_step(state, pending).await?;
            steps += 1;
        };

        for middleware in &self.middlewares {
            middleware.after_agent(state).await?;
        }

        Ok(usage)
    }

    /// Builds the request for one step from the base request.
    fn build_request(&self, state: &AgentState) -> ChatRequest {
        let mut request = self.request.clone();
        request.messages = state.messages.clone();
        request.tools = self.tools.specs();
        request
    }

    /// Persists the new conversation messages accumulated for this step.
    async fn persist_step(
        &self,
        state: &AgentState,
        pending: &mut Vec<Message>,
    ) -> Result<(), Error> {
        if !pending.is_empty()
            && let Some(cp) = &self.checkpointer
            && let Some(tid) = state.context.get::<ThreadId>()
        {
            let run_id = state
                .context
                .get::<RunId>()
                .cloned()
                .unwrap_or_else(|| RunId(generate_id()));
            let stored = pending
                .iter()
                .map(|message| CheckpointMessage::from_message(message, &tid.0, &run_id.0))
                .collect();
            cp.append(&tid.0, stored).await?;
        }
        pending.clear();
        Ok(())
    }

    /// Returns the long-term store, if configured.
    pub fn store(&self) -> Option<&Arc<dyn Store>> {
        self.store.as_ref()
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful assistant.";

/// Installs the configured system prompt as the first message of `messages`.
///
/// The system prompt always leads the conversation, so it is placed at index 0
/// rather than appended after the history loaded from a
/// [`Checkpointer`](crate::Checkpointer). A leading system message that is
/// already present is replaced in place, so a second run on the same thread
/// does not accumulate a duplicate system message.
fn apply_system_prompt(messages: &mut Vec<Message>, prompt: &str) {
    match messages.first_mut() {
        Some(first) if first.role == Role::System => {
            first.content = Content::Text(prompt.to_string())
        }
        _ => messages.insert(0, Message::system(prompt)),
    }
}

/// Whether the named tool is declared `return_direct`.
fn returns_direct(tools: &ToolSet, name: &str) -> bool {
    tools
        .get(name)
        .is_some_and(|tool| tool.spec().return_direct)
}

/// The user-facing final answer built from a `return_direct` tool result.
///
/// Tool results are JSON-encoded for the model, so a plain string answer is
/// unwrapped here; anything else (numbers, objects) stays as it is.
fn direct_answer(content: &str) -> String {
    serde_json::from_str::<String>(content).unwrap_or_else(|_| content.to_string())
}

/// One tool call's outcome, tagged with its position in the model's batch.
struct ToolOutcome {
    /// Position of the call within the assistant message's `tool_calls`.
    index: usize,
    /// The call that produced this outcome.
    call: ToolCall,
    /// The tool's result, or the error it failed with.
    result: Result<String, Error>,
}

/// Runs a batch of tool calls concurrently — up to `concurrency` in flight —
/// yielding each outcome as soon as its call finishes.
///
/// Outcomes arrive in completion order; [`ToolOutcome::index`] lets callers
/// restore the model's request order afterwards.
fn execute_tool_batch(
    tool: Arc<dyn ToolHandler>,
    calls: Vec<ToolCall>,
    concurrency: usize,
) -> impl Stream<Item = ToolOutcome> {
    async_stream::stream! {
        let mut queue = calls.into_iter().enumerate();
        let mut pending = FuturesUnordered::new();

        for(index, call) in queue.by_ref().take(concurrency.max(1)) {
            pending.push(run_tool_call(Arc::clone(&tool), index, call));
        }

        // Each time a call finishes, start the next one and yield the outcome.
        while let Some(outcome) = pending.next().await {
            yield outcome;
            if let Some((index, call)) = queue.next() {
                pending.push(run_tool_call(Arc::clone(&tool), index, call));
            }
        }
    }
}

/// Runs one tool call, pairing its outcome with its batch position.
async fn run_tool_call(tool: Arc<dyn ToolHandler>, index: usize, call: ToolCall) -> ToolOutcome {
    let result = tool.handle(call.clone()).await;
    ToolOutcome {
        index,
        call,
        result,
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
    checkpointer: Option<Arc<dyn Checkpointer>>,
    store: Option<Arc<dyn Store>>,
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

    /// Sets the leading system prompt for every run.
    ///
    /// Without an explicit prompt, the agent keeps a leading system message
    /// from the conversation or uses "You are a helpful assistant."
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

    /// Sets how many tool calls from one model response may run concurrently.
    ///
    /// `1` runs them sequentially; values above one enable parallel execution.
    pub fn max_concurrent_tool_calls(mut self, limit: usize) -> Self {
        self.config.max_concurrent_tool_calls = limit;
        self
    }

    /// Sets the checkpointer that persists and restores conversation history.
    pub fn checkpointer(mut self, cp: impl Checkpointer + 'static) -> Self {
        self.checkpointer = Some(Arc::new(cp));
        self
    }

    /// Enables long-term memory: stores `store` on the agent and registers the
    /// `remember` / `recall` tools bound to it.
    pub fn store(mut self, store: impl Store + 'static) -> Self {
        self.store = Some(Arc::new(store));
        self
    }

    /// Builds the agent.
    ///
    /// Fails with [`Error::MissingSetting`] if the model or base request was not
    /// provided.
    pub fn build(mut self) -> Result<Agent, Error> {
        let model = self.model.ok_or(Error::MissingSetting { name: "model" })?;
        let request = self
            .request
            .ok_or(Error::MissingSetting { name: "request" })?;

        // Register the `remember` / `recall` tools when a store is configured.
        if let Some(store) = self.store.clone() {
            self.tools = self
                .tools
                .with_tool(RememberTool {
                    store: Arc::clone(&store),
                })
                .with_tool(RecallTool { store });
        }

        Ok(Agent {
            model,
            request,
            tools: self.tools,
            system_prompt: self.system_prompt,
            middlewares: self.middlewares,
            config: self.config,
            checkpointer: self.checkpointer,
            store: self.store,
        })
    }
}

/// Generates a run id. Currently a uuid v7; may be swapped for a shorter id.
fn generate_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Adds a runtime-produced message to the conversation and the pending checkpoint.
fn record_message(state: &mut AgentState, pending: &mut Vec<Message>, message: Message) {
    pending.push(message.clone());
    state.messages.push(message);
}
