use async_trait::async_trait;
use sqlx::QueryBuilder;
use sqlx::postgres::PgPool;

use crate::Error;
use crate::checkpointer::sql_common::{
    DatabasePool, DatabaseTransaction, MessageRow, flag, insert_message, migrate, next_seq,
    persistence, row_to_message,
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
        migrate(DatabasePool::Postgres(&self.pool), DDL).await
    }
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
        let mut next = next_seq(DatabaseTransaction::Postgres(&mut tx), thread_id).await?;

        for mut message in messages {
            message.thread_id = thread_id.to_string();
            message.seq = next;
            next += 1;
            insert_message(DatabaseTransaction::Postgres(&mut tx), &message).await?;
        }

        tx.commit().await.map_err(persistence)?;
        Ok(())
    }

    async fn set_context(
        &self,
        thread_id: &str,
        ids: &[String],
        is_context: bool,
    ) -> Result<(), Error> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut qb = QueryBuilder::new("UPDATE messages SET is_context = ");
        qb.push_bind(flag(is_context));
        qb.push(" WHERE thread_id = ");
        qb.push_bind(thread_id);
        qb.push(" AND id IN (");
        for (i, id) in ids.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            qb.push_bind(id);
        }
        qb.push(")");
        qb.build().execute(&self.pool).await.map_err(persistence)?;
        Ok(())
    }
}
