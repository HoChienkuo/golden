use async_trait::async_trait;
use sqlx::MySql;
use sqlx::mysql::MySqlPool;

use crate::Error;
use crate::checkpointer::sql_common::{
    MessageRow, content_json, flag, persistence, role_str, row_to_message, summary_of_json,
    tool_calls_json, validate_compact,
};
use crate::checkpointer::{CheckpointMessage, Checkpointer};

const DDL: &str = include_str!("mysql_messages.sql");

/// The batch size of the `id IN (?, ...)` id list in [`MySqlCheckpointer::compact`].
///
/// Stays far below MySQL's 65535 bind-parameter limit. It also keeps the query
/// text short: statements built with [`sqlx::QueryBuilder`] vary in length with
/// the batch size, so they must never be marked persistent (see
/// [`dynamic`]).
///
/// [`dynamic`]: sqlx::QueryBuilder#note-reuse
const ID_BATCH: usize = 256;

/// A MySQL-backed [`Checkpointer`], gated on the `persistence-mysql` feature.
pub struct MySqlCheckpointer {
    pool: MySqlPool,
}

impl MySqlCheckpointer {
    /// Wraps an existing pool. The schema must already exist
    /// (call [`Self::migrate`] or run the DDL yourself).
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Opens a connection pool to `url` and wraps it.
    ///
    /// `url` is a MySQL URL like
    /// `mysql://user:password@localhost:3306/dbname`.
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let pool = MySqlPool::connect(url).await.map_err(persistence)?;
        Ok(Self::new(pool))
    }

    /// Creates the `messages` table and index if they do not exist.
    pub async fn migrate(&self) -> Result<(), Error> {
        for stmt in DDL.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(persistence)?;
        }
        Ok(())
    }
}

async fn next_seq(tx: &mut sqlx::Transaction<'_, MySql>, thread_id: &str) -> Result<u64, Error> {
    let max_seq: Option<i64> =
        sqlx::query_scalar("SELECT MAX(seq) FROM messages WHERE thread_id = ?")
            .bind(thread_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(persistence)?
            .flatten();
    Ok(max_seq.map_or(0, |m| (m + 1) as u64))
}

async fn insert_message(
    tx: &mut sqlx::Transaction<'_, MySql>,
    message: &CheckpointMessage,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        INSERT INTO messages
            (id, thread_id, run_id, seq, role, content,
             tool_calls, tool_call_id, created_at,
             is_context, is_summary, summary_of)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&message.id)
    .bind(&message.thread_id)
    .bind(&message.run_id)
    .bind(message.seq as i64)
    .bind(role_str(message.role))
    .bind(content_json(message)?)
    .bind(tool_calls_json(message)?)
    .bind(&message.tool_call_id)
    .bind(message.created_at)
    .bind(flag(message.is_context))
    .bind(flag(message.is_summary))
    .bind(summary_of_json(message)?)
    .execute(&mut **tx)
    .await
    .map_err(persistence)?;
    Ok(())
}

/// Hides one batch of covered messages and returns how many rows changed.
///
/// The statement is built with [`sqlx::QueryBuilder`], so it is marked
/// non-persistent: the placeholder count changes with the batch size, which
/// would otherwise feed every batch into the connection's prepared-statement
/// cache (see [`MySqlCheckpointer`] and the sqlx `QueryBuilder` docs).
async fn hide_batch(
    tx: &mut sqlx::Transaction<'_, MySql>,
    thread_id: &str,
    batch: &[String],
) -> Result<u64, Error> {
    let mut qb = sqlx::QueryBuilder::new("UPDATE messages SET is_context = 0 WHERE thread_id = ");
    qb.push_bind(thread_id);
    qb.push(" AND id IN (");
    for (i, id) in batch.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push_bind(id);
    }
    qb.push(")");
    let result = qb
        .build()
        .persistent(false)
        .execute(&mut **tx)
        .await
        .map_err(persistence)?;
    Ok(result.rows_affected())
}

#[async_trait]
impl Checkpointer for MySqlCheckpointer {
    async fn load(&self, thread_id: &str) -> Result<Vec<CheckpointMessage>, Error> {
        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, thread_id, run_id, seq, role, content,
                   tool_calls, tool_call_id, created_at,
                   is_context, is_summary, summary_of
            FROM messages
            WHERE thread_id = ? AND is_context = 1
            ORDER BY created_at ASC, seq ASC
            "#,
        )
        .bind(thread_id)
        .fetch_all(&self.pool)
        .await
        .map_err(persistence)?;

        rows.into_iter()
            .map(row_to_message)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn append(&self, thread_id: &str, messages: Vec<CheckpointMessage>) -> Result<(), Error> {
        if messages.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(persistence)?;
        let mut next = next_seq(&mut tx, thread_id).await?;

        for mut message in messages {
            message.thread_id = thread_id.to_string();
            message.seq = next;
            next += 1;
            insert_message(&mut tx, &message).await?;
        }

        tx.commit().await.map_err(persistence)?;
        Ok(())
    }

    async fn compact(
        &self,
        thread_id: &str,
        ids: &[String],
        summary: CheckpointMessage,
    ) -> Result<(), Error> {
        validate_compact(ids, &summary)?;

        let mut tx = self.pool.begin().await.map_err(persistence)?;

        // Hide the covered messages.
        let mut hidden = 0u64;
        for batch in ids.chunks(ID_BATCH) {
            hidden += hide_batch(&mut tx, thread_id, batch).await?;
        }

        // Every id must have matched; otherwise a typo would silently drop
        // messages from the covered range.
        if hidden != ids.len() as u64 {
            return Err(persistence(format!(
                "compact: expected to hide {} rows, hid {}",
                ids.len(),
                hidden
            )));
        }

        // The summary covers exactly `ids` and takes the next sequence slot.
        let mut summary = summary;
        summary.thread_id = thread_id.to_string();
        summary.summary_of = Some(ids.to_vec());
        summary.seq = next_seq(&mut tx, thread_id).await?;

        insert_message(&mut tx, &summary).await?;

        tx.commit().await.map_err(persistence)?;
        Ok(())
    }
}
