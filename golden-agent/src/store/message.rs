use serde::{Deserialize, Serialize};

use crate::llm::chat::{Content, Message, Role, ToolCall};

/// A persisted message with its metadata.
///
/// One row in the `messages` table. The provider-neutral [`Message`] is what
/// the model sees; [`StoredMessage`] wraps it with the metadata persistence
/// needs: identity, ordering, and the two flags that drive context
/// visibility and summarization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMessage {
    /// The unique id of this record (uuid v7 / ulid).
    ///
    /// Immutable. Referenced by [`Self::summary_of`] and used to identify
    /// which rows a compaction covers.
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
    ///
    /// Rewritable: a summary message takes the `created_at` of the last
    /// message it covers, so it sorts right before the messages that follow
    /// the covered range.
    pub created_at: i64,

    /// Whether this message is fed to the model as context.
    ///
    /// `true` (the default) for normal messages. Compaction sets it to
    /// `false` for the messages a summary replaces.
    pub is_context: bool,

    /// Whether this message is a summary produced by compaction.
    ///
    /// `false` for normal messages. `true` for summary messages, which are
    /// always `is_context = true` until a later compaction rolls them up.
    pub is_summary: bool,

    /// Ids of the messages this summary covers.
    ///
    /// `None` for normal messages. `Some(ids)` for summaries; used for
    /// audit, undo, and re-compaction.
    pub summary_of: Option<Vec<String>>,
}

impl StoredMessage {
    /// Wraps a plain message with its persistence metadata.
    ///
    /// `is_summary` is `false` and `summary_of` is `None`. Callers assign
    /// `id`, `run_id`, `seq`, and `created_at` from their own id / clock /
    /// counter.
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
            is_summary: false,
            summary_of: None,
        }
    }

    /// Wraps a summary message covering the messages with the given ids.
    ///
    /// Reserved for compaction. `created_at` should be the `created_at` of the
    /// last covered message so the summary sorts right before the messages
    /// that follow the covered range.
    pub fn summary(
        id: impl Into<String>,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        seq: u64,
        content: Content,
        created_at: i64,
        summary_of: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            thread_id: thread_id.into(),
            run_id: run_id.into(),
            seq,
            role: Role::System,
            content,
            tool_calls: Vec::new(),
            tool_call_id: None,
            created_at,
            is_context: true,
            is_summary: true,
            summary_of: Some(summary_of),
        }
    }

    /// Converts a [`Message`] to a stored row, generating a fresh id and
    /// timestamp. `seq` is left at 0 and assigned later by
    /// [`Checkpointer::append`](crate::Checkpointer::append).
    pub fn from_message(msg: &Message, thread_id: &str, run_id: &str) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            thread_id: thread_id.to_string(),
            run_id: run_id.to_string(),
            seq: 0,
            role: msg.role,
            content: msg.content.clone(),
            tool_calls: msg.tool_calls.clone(),
            tool_call_id: msg.tool_call_id.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            is_context: true,
            is_summary: false,
            summary_of: None,
        }
    }
}

impl From<StoredMessage> for Message {
    /// Extracts the model-facing message.
    ///
    /// The persistence metadata (`id`, `run_id`, `seq`, `created_at`,
    /// `is_context`, `is_summary`, `summary_of`) is dropped.
    fn from(stored: StoredMessage) -> Self {
        Message {
            role: stored.role,
            content: stored.content,
            name: None,
            tool_calls: stored.tool_calls,
            tool_call_id: stored.tool_call_id,
        }
    }
}
