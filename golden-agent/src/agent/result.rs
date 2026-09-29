use crate::llm::chat::{Message, Role};

use super::state::AgentState;

/// The outcome of an [`Agent::invoke`](crate::Agent::invoke) run.
#[derive(Debug, Clone)]
pub struct AgentResult {
    state: AgentState,
}

impl AgentResult {
    /// Wraps the finished state.
    pub(crate) fn new(state: AgentState) -> Self {
        Self { state }
    }

    /// The full conversation, including tool calls and their results.
    pub fn messages(&self) -> &[Message] {
        &self.state.messages
    }

    /// The final assistant message, if the run produced one.
    pub fn message(&self) -> Option<&Message> {
        self.state
            .messages
            .iter()
            .rev()
            .find(|message| message.role == Role::Assistant)
    }

    /// The final assistant text, or an empty string.
    pub fn text(&self) -> &str {
        self.message().map(Message::text).unwrap_or_default()
    }

    /// Consumes the result, returning the conversation.
    pub fn into_messages(self) -> Vec<Message> {
        self.state.messages
    }
}
