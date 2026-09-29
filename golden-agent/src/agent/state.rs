use crate::llm::chat::Message;

/// The mutable conversation carried through an agent run.
///
/// Currently it holds only the message list; more fields may be added later to
/// support richer middleware.
#[derive(Debug, Clone, Default)]
pub struct AgentState {
    /// The full conversation: system, user, assistant, and tool messages.
    pub messages: Vec<Message>,
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
