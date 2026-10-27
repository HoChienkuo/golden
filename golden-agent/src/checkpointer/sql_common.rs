use crate::Error;
use crate::checkpointer::CheckpointMessage;
use crate::llm::chat::Role;

/// A connection pool for one of the supported SQL databases.
#[derive(Clone, Copy)]
pub(crate) enum DatabasePool<'a> {
    #[cfg(feature = "persistence-mysql")]
    MySql(&'a sqlx::MySqlPool),
    #[cfg(feature = "persistence-postgres")]
    Postgres(&'a sqlx::PgPool),
    #[cfg(feature = "persistence-sqlite")]
    Sqlite(&'a sqlx::SqlitePool),
}

/// A transaction for one of the supported SQL databases.
pub(crate) enum DatabaseTransaction<'a, 'c> {
    #[cfg(feature = "persistence-mysql")]
    MySql(&'a mut sqlx::Transaction<'c, sqlx::MySql>),
    #[cfg(feature = "persistence-postgres")]
    Postgres(&'a mut sqlx::Transaction<'c, sqlx::Postgres>),
    #[cfg(feature = "persistence-sqlite")]
    Sqlite(&'a mut sqlx::Transaction<'c, sqlx::Sqlite>),
}

/// Executes the statements in a database schema definition.
pub(crate) async fn migrate(pool: DatabasePool<'_>, ddl: &'static str) -> Result<(), Error> {
    for statement in ddl.split(';').map(str::trim).filter(|sql| !sql.is_empty()) {
        match pool {
            #[cfg(feature = "persistence-mysql")]
            DatabasePool::MySql(pool) => sqlx::query(statement).execute(pool).await.map(|_| ()),
            #[cfg(feature = "persistence-postgres")]
            DatabasePool::Postgres(pool) => sqlx::query(statement).execute(pool).await.map(|_| ()),
            #[cfg(feature = "persistence-sqlite")]
            DatabasePool::Sqlite(pool) => sqlx::query(statement).execute(pool).await.map(|_| ()),
        }
        .map_err(persistence)?;
    }
    Ok(())
}

/// Returns the next sequence number for a thread.
pub(crate) async fn next_seq(
    tx: DatabaseTransaction<'_, '_>,
    thread_id: &str,
) -> Result<u64, Error> {
    let max_seq: Option<i64> = match tx {
        #[cfg(feature = "persistence-mysql")]
        DatabaseTransaction::MySql(tx) => {
            sqlx::query_scalar("SELECT MAX(seq) FROM messages WHERE thread_id = ?")
                .bind(thread_id)
                .fetch_optional(&mut **tx)
                .await
        }
        #[cfg(feature = "persistence-postgres")]
        DatabaseTransaction::Postgres(tx) => {
            sqlx::query_scalar("SELECT MAX(seq) FROM messages WHERE thread_id = $1")
                .bind(thread_id)
                .fetch_optional(&mut **tx)
                .await
        }
        #[cfg(feature = "persistence-sqlite")]
        DatabaseTransaction::Sqlite(tx) => {
            sqlx::query_scalar("SELECT MAX(seq) FROM messages WHERE thread_id = ?")
                .bind(thread_id)
                .fetch_optional(&mut **tx)
                .await
        }
    }
    .map_err(persistence)?
    .flatten();

    Ok(max_seq.map_or(0, |max| (max + 1) as u64))
}

macro_rules! insert_message {
    ($tx:expr, $sql:expr, $message:expr, $content:expr, $tool_calls:expr) => {
        sqlx::query($sql)
            .bind(&$message.id)
            .bind(&$message.thread_id)
            .bind(&$message.run_id)
            .bind($message.seq as i64)
            .bind(role_str($message.role))
            .bind($content)
            .bind($tool_calls)
            .bind(&$message.tool_call_id)
            .bind($message.created_at)
            .bind(flag($message.is_context))
            .execute(&mut **$tx)
            .await
            .map(|_| ())
    };
}

/// Inserts a message using the target database's placeholder syntax.
pub(crate) async fn insert_message(
    tx: DatabaseTransaction<'_, '_>,
    message: &CheckpointMessage,
) -> Result<(), Error> {
    let content = content_json(message)?;
    let tool_calls = tool_calls_json(message)?;

    match tx {
        #[cfg(feature = "persistence-mysql")]
        DatabaseTransaction::MySql(tx) => insert_message!(
            tx,
            "INSERT INTO messages (id, thread_id, run_id, seq, role, content, tool_calls, \
             tool_call_id, created_at, is_context) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            message,
            content,
            tool_calls
        ),
        #[cfg(feature = "persistence-postgres")]
        DatabaseTransaction::Postgres(tx) => insert_message!(
            tx,
            "INSERT INTO messages (id, thread_id, run_id, seq, role, content, tool_calls, \
             tool_call_id, created_at, is_context) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            message,
            content,
            tool_calls
        ),
        #[cfg(feature = "persistence-sqlite")]
        DatabaseTransaction::Sqlite(tx) => insert_message!(
            tx,
            "INSERT INTO messages (id, thread_id, run_id, seq, role, content, tool_calls, \
             tool_call_id, created_at, is_context) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            message,
            content,
            tool_calls
        ),
    }
    .map_err(persistence)?;

    Ok(())
}

/// Maps any displayable error into [`Error::Persistence`].
pub(crate) fn persistence(err: impl std::fmt::Display) -> Error {
    Error::Persistence {
        message: err.to_string(),
    }
}

/// A message row as stored in the `messages` table.
///
/// Field order matches every backend's `SELECT` list, so `FromRow` derives and
/// `try_get` by index both line up with the schema. `is_context`
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

/// The wire type of the two boolean-ish columns.
pub(crate) fn flag(value: bool) -> i64 {
    i64::from(value)
}
