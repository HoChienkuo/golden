use crate::llm::chat::Message;

use super::context::Context;

/// The mutable state carried through an agent run.
#[derive(Debug, Clone, Default)]
pub struct AgentState {
    /// The full conversation: system, user, assistant, and tool messages.
    ///
    /// Changes made by middleware affect the live conversation. Checkpoints
    /// save the original input and messages produced by the agent.
    pub messages: Vec<Message>,
    /// The read-only inputs of this run, supplied by the caller.
    pub context: Context,
}

impl AgentState {
    /// Creates an empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the most recent message, if any.
    pub fn last(&self) -> Option<&Message> {
        self.messages.last()
    }
}
