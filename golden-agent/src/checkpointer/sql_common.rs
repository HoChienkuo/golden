use crate::Error;
use crate::checkpointer::CheckpointMessage;
use crate::llm::chat::Role;

/// Maps any displayable error into [`Error::Persistence`].
pub(crate) fn persistence(err: impl std::fmt::Display) -> Error {
    Error::Persistence {
        message: err.to_string(),
    }
}

/// A message row as stored in the `messages` table.
///
/// Field order matches every backend's `SELECT` list, so `FromRow` derives and
/// `try_get` by index both line up with the schema. `is_context` / `is_summary`
/// are integers because SQLite has no boolean type; the other backends read
/// their `BOOLEAN` columns into the same fields.
#[derive(sqlx::FromRow)]
pub(crate) struct MessageRow {
    pub id: String,
    pub thread_id: String,
    pub run_id: String,
    pub seq: i64,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub tool_call_id: Option<String>,
    pub created_at: i64,
    pub is_context: i64,
    pub is_summary: i64,
    pub summary_of: Option<String>,
}

/// Reconstructs a [`CheckpointMessage`] from its stored row.
pub(crate) fn row_to_message(row: MessageRow) -> Result<CheckpointMessage, Error> {
    let MessageRow {
        id,
        thread_id,
        run_id,
        seq,
        role,
        content,
        tool_calls,
        tool_call_id,
        created_at,
        is_context,
        is_summary,
        summary_of,
    } = row;
    Ok(CheckpointMessage {
        id,
        thread_id,
        run_id,
        seq: seq as u64,
        role: parse_role(&role)?,
        content: serde_json::from_str(&content).map_err(persistence)?,
        tool_calls: match tool_calls {
            Some(json) => serde_json::from_str(&json).map_err(persistence)?,
            None => Vec::new(),
        },
        tool_call_id,
        created_at,
        is_context: is_context != 0,
        is_summary: is_summary != 0,
        summary_of: match summary_of {
            Some(json) => Some(serde_json::from_str(&json).map_err(persistence)?),
            None => None,
        },
    })
}

/// Parses the stored role name back into a [`Role`].
pub(crate) fn parse_role(s: &str) -> Result<Role, Error> {
    match s {
        "system" => Ok(Role::System),
        "user" => Ok(Role::User),
        "assistant" => Ok(Role::Assistant),
        "tool" => Ok(Role::Tool),
        other => Err(persistence(format!("unknown role: {other}"))),
    }
}

/// The stored role name.
pub(crate) fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

/// The `content` column value of a message.
pub(crate) fn content_json(message: &CheckpointMessage) -> Result<String, Error> {
    serde_json::to_string(&message.content).map_err(persistence)
}

/// The `tool_calls` column value of a message.
pub(crate) fn tool_calls_json(message: &CheckpointMessage) -> Result<String, Error> {
    serde_json::to_string(&message.tool_calls).map_err(persistence)
}

/// The `summary_of` column value of a message, or `NULL` for normal messages.
pub(crate) fn summary_of_json(message: &CheckpointMessage) -> Result<Option<String>, Error> {
    message
        .summary_of
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(persistence)
}

/// Validates the invariants of [`Checkpointer::compact`](crate::Checkpointer::compact)
/// before any row is touched.
pub(crate) fn validate_compact(ids: &[String], summary: &CheckpointMessage) -> Result<(), Error> {
    if ids.is_empty() {
        return Err(persistence("compact: ids must not be empty"));
    }

    // The summary must be visible context and flagged as a summary.
    if !summary.is_summary || !summary.is_context {
        return Err(persistence(
            "compact: summary must have is_summary = true and is_context = true",
        ));
    }
    Ok(())
}

/// The wire type of the two boolean-ish columns.
pub(crate) fn flag(value: bool) -> i64 {
    i64::from(value)
}
