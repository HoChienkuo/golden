use async_trait::async_trait;

use crate::error::Error;
use crate::llm::chat::{Content, Message, Role};

use super::middleware::Middleware;
use super::state::AgentState;

/// Rewrites the system prompt before every model call.
///
/// The closure receives the current [`AgentState`] — its messages and its
/// [`context`](AgentState::context) — and returns the system prompt to use. It
/// runs on the `before_model` hook, so it applies to both
/// [`Agent::invoke`](crate::Agent::invoke) and [`Agent::stream`](crate::Agent::stream).
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
        let prompt = (self.render)(state);

        match state.messages.first_mut() {
            Some(message) if message.role == Role::System => {
                message.content = Content::Text(prompt);
            }
            _ => state.messages.insert(0, Message::system(prompt)),
        }

        Ok(())
    }
}
