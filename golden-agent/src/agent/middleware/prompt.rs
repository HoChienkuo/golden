use async_trait::async_trait;

use crate::agent::AgentState;
use crate::error::Error;
use crate::llm::chat::{Content, Role};

use super::middleware::Middleware;

/// Rewrites the system prompt before every model call.
///
/// The closure receives the current [`AgentState`] — its messages and its
/// [`context`](AgentState::context) — and returns the system prompt to use. It
/// runs on the `before_model` hook, so it applies to both
/// [`Agent::invoke`](crate::Agent::invoke) and [`Agent::stream`](crate::Agent::stream).
/// It replaces the leading system message initialized by the agent.
///
/// ```
/// use golden_agent::{AgentState, DynamicPrompt};
///
/// let prompt = DynamicPrompt::new(|state: &AgentState| {
///     format!("Answer in at most {} turns.", state.messages.len())
/// });
/// # let _ = prompt;
/// ```
pub struct DynamicPrompt<F> {
    render: F,
}

impl<F> DynamicPrompt<F> {
    /// Creates a dynamic-prompt middleware from a render function.
    pub fn new(render: F) -> Self {
        Self { render }
    }
}

#[async_trait]
impl<F> Middleware for DynamicPrompt<F>
where
    F: Fn(&AgentState) -> String + Send + Sync,
{
    async fn before_model(&self, state: &mut AgentState) -> Result<(), Error> {
        if !state
            .messages
            .first()
            .is_some_and(|message| message.role == Role::System)
        {
            return Err(Error::InvalidRequest {
                message: "DynamicPrompt requires a leading system message".to_string(),
            });
        }

        let prompt = (self.render)(state);
        state.messages[0].content = Content::Text(prompt);

        Ok(())
    }
}
