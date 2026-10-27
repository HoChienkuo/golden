use async_trait::async_trait;
use sqlx::Postgres;
use sqlx::postgres::PgPool;

use crate::Error;
use crate::checkpointer::sql_common::{
    MessageRow, content_json, flag, persistence, role_str, row_to_message, tool_calls_json,
};
use crate::checkpointer::{CheckpointMessage, Checkpointer};

const DDL: &str = include_str!("postgres_messages.sql");

/// A PostgreSQL-backed [`Checkpointer`], gated on the `persistence-postgres` feature.
pub struct PostgresCheckpointer {
    pool: PgPool,
}

impl PostgresCheckpointer {
    /// Wraps an existing pool. The schema must already exist
    /// (call [`Self::migrate`] or run the DDL yourself).
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Opens a connection pool to `url` and wraps it.
    ///
    /// `url` is a PostgreSQL URL like
    /// `postgres://user:password@localhost:5432/dbname`.
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let pool = PgPool::connect(url).await.map_err(persistence)?;
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

async fn next_seq(tx: &mut sqlx::Transaction<'_, Postgres>, thread_id: &str) -> Result<u64, Error> {
    let max_seq: Option<i64> =
        sqlx::query_scalar("SELECT MAX(seq) FROM messages WHERE thread_id = $1")
            .bind(thread_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(persistence)?
            .flatten();
    Ok(max_seq.map_or(0, |m| (m + 1) as u64))
}

async fn insert_message(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    message: &CheckpointMessage,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        INSERT INTO messages
            (id, thread_id, run_id, seq, role, content,
             tool_calls, tool_call_id, created_at,
             is_context)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
    .execute(&mut **tx)
    .await
    .map_err(persistence)?;
    Ok(())
}

#[async_trait]
impl Checkpointer for PostgresCheckpointer {
    async fn load(&self, thread_id: &str) -> Result<Vec<CheckpointMessage>, Error> {
        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, thread_id, run_id, seq, role, content,
                   tool_calls, tool_call_id, created_at,
                   is_context
            FROM messages
            WHERE thread_id = $1 AND is_context = TRUE
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
}
