use serde::{Deserialize, Serialize};

use crate::llm::chat::{Content, Message, Role, ToolCall};

/// A persisted message with its metadata.
///
/// One row in the `messages` table. The provider-neutral [`Message`] is what
/// the model sees; [`CheckpointMessage`] wraps it with the metadata persistence
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointMessage {
    /// The unique id of this record (uuid v7 / ulid).
    pub id: String,

    /// The conversation this message belongs to.
    pub thread_id: String,

    /// The agent run that produced this message.
    ///
    /// One `invoke` / `stream` call = one `run_id`. Consecutive messages
    /// sharing a `run_id` form a single turn (user -> assistant -> tool ...),
    /// which compaction strategies use to keep turns intact.
    pub run_id: String,

    /// Monotonic sequence number within the thread.
    ///
    /// Strictly increasing per `thread_id`. Used as a tiebreaker when two
    /// messages share a `created_at`.
    pub seq: u64,

    /// The role of the message author.
    pub role: Role,

    /// The message content.
    pub content: Content,

    /// The tool calls requested by the model (assistant messages).
    pub tool_calls: Vec<ToolCall>,

    /// The id of the tool call this message responds to (tool messages).
    pub tool_call_id: Option<String>,

    /// The creation timestamp (unix seconds).
    pub created_at: i64,

    /// Whether this message is fed to the model as context.
    ///
    /// `true` (the default) for normal messages.
    pub is_context: bool,
}

impl CheckpointMessage {
    /// Wraps a plain message with its persistence metadata.
    pub fn new(
        id: impl Into<String>,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        seq: u64,
        message: Message,
        created_at: i64,
    ) -> Self {
        Self {
            id: id.into(),
            thread_id: thread_id.into(),
            run_id: run_id.into(),
            seq,
            role: message.role,
            content: message.content,
            tool_calls: message.tool_calls,
            tool_call_id: message.tool_call_id,
            created_at,
            is_context: true,
        }
    }

    /// Converts a [`Message`] to a stored row, generating a fresh id and
    /// timestamp. `seq` is left at 0 and assigned later by
    /// [`Checkpointer::append`](crate::Checkpointer::append).
    pub fn from_message(msg: &Message, thread_id: &str, run_id: &str) -> Self {
        Self {
            id: msg
                .id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::now_v7().to_string()),
            thread_id: thread_id.to_string(),
            run_id: msg.run_id.clone().unwrap_or_else(|| run_id.to_string()),
            seq: 0,
            role: msg.role,
            content: msg.content.clone(),
            tool_calls: msg.tool_calls.clone(),
            tool_call_id: msg.tool_call_id.clone(),
            created_at: msg.created_at.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0)
            }),
            is_context: msg.is_context,
        }
    }
}

impl From<CheckpointMessage> for Message {
    /// Extracts the model-facing message.
    ///
    /// The persistence metadata (`id`, `run_id`, `seq`, `created_at`,
    /// `is_context`) is dropped.
    fn from(stored: CheckpointMessage) -> Self {
        Message {
            role: stored.role,
            content: stored.content,
            name: None,
            tool_calls: stored.tool_calls,
            tool_call_id: stored.tool_call_id,
            id: Some(stored.id),
            run_id: Some(stored.run_id),
            is_context: stored.is_context,
            created_at: Some(stored.created_at),
        }
    }
}
