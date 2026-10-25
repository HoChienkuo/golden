use crate::Error;
use crate::store::Checkpointer;
use crate::{Role, StoredMessage};
use async_trait::async_trait;
use sqlx::FromRow;
use sqlx::QueryBuilder;
use sqlx::sqlite::SqlitePool;

const DDL: &str = include_str!("sqlite_messages.sql");

pub struct SqliteCheckpointer {
    pool: SqlitePool,
}

impl SqliteCheckpointer {
    /// Wraps an existing pool. The schema must already exist
    /// (call [`Self::migrate`] or run the DDL yourself).
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Opens a connection pool to `url` and wraps it.
    ///
    /// `url` is a SQLite URL like `sqlite:path/to.db` or `sqlite::memory:`.
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let pool = SqlitePool::connect(url).await.map_err(persistence)?;
        Ok(Self::new(pool))
    }

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

/// Maps any displayable error into [`Error::Persistence`].
fn persistence(err: impl std::fmt::Display) -> Error {
    Error::Persistence {
        message: err.to_string(),
    }
}

#[derive(FromRow)]
struct MessageRow {
    id: String,
    thread_id: String,
    run_id: String,
    seq: i64,
    role: String,
    content: String,
    tool_calls: Option<String>,
    tool_call_id: Option<String>,
    created_at: i64,
    is_context: i64,
    is_summary: i64,
    summary_of: Option<String>,
}

fn row_to_message(row: MessageRow) -> Result<StoredMessage, Error> {
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
    Ok(StoredMessage {
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

fn parse_role(s: &str) -> Result<Role, Error> {
    match s {
        "system" => Ok(Role::System),
        "user" => Ok(Role::User),
        "assistant" => Ok(Role::Assistant),
        "tool" => Ok(Role::Tool),
        other => Err(Error::Persistence {
            message: format!("unknown role: {other}"),
        }),
    }
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

async fn next_seq(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    thread_id: &str,
) -> Result<u64, Error> {
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
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    message: &StoredMessage,
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
    .bind(serde_json::to_string(&message.content).map_err(persistence)?)
    .bind(serde_json::to_string(&message.tool_calls).map_err(persistence)?)
    .bind(&message.tool_call_id)
    .bind(message.created_at)
    .bind(message.is_context as i64)
    .bind(message.is_summary as i64)
    .bind(
        message
            .summary_of
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(persistence)?,
    )
    .execute(&mut **tx) // ← &mut **tx
    .await
    .map_err(persistence)?;
    Ok(())
}

#[async_trait]
impl Checkpointer for SqliteCheckpointer {
    async fn load(&self, thread_id: &str) -> Result<Vec<StoredMessage>, Error> {
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

    async fn append(&self, thread_id: &str, messages: Vec<StoredMessage>) -> Result<(), Error> {
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
        summary: StoredMessage,
    ) -> Result<(), Error> {
        // ── 校验入参 ────────────────────────────────────────────────
        if ids.is_empty() {
            return Err(Error::Persistence {
                message: "compact: ids must not be empty".to_string(),
            });
        }

        // 不变量：摘要必须 is_summary=true 且 is_context=true
        if !summary.is_summary || !summary.is_context {
            return Err(Error::Persistence {
                message: "compact: summary must have is_summary = true and is_context = true"
                    .to_string(),
            });
        }

        // ── 事务开始 ────────────────────────────────────────────────
        let mut tx = self.pool.begin().await.map_err(persistence)?;

        // ① 隐藏被覆盖的消息（用 QueryBuilder 安全构造动态 IN 子句）
        let mut qb = QueryBuilder::new("UPDATE messages SET is_context = 0 WHERE thread_id = ");
        qb.push_bind(thread_id);
        qb.push(" AND id IN (");
        for (i, id) in ids.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            qb.push_bind(id);
        }
        qb.push(")");
        let result = qb.build().execute(&mut *tx).await.map_err(persistence)?;

        // ② 校验：所有 ids 都真的存在（防止传错 id 导致静默丢失）
        if result.rows_affected() != ids.len() as u64 {
            return Err(Error::Persistence {
                message: format!(
                    "compact: expected to hide {} rows, hid {}",
                    ids.len(),
                    result.rows_affected()
                ),
            });
        }

        // ③ 准备摘要：强制覆盖 thread_id / summary_of / seq
        let mut summary = summary;
        summary.thread_id = thread_id.to_string();
        summary.summary_of = Some(ids.to_vec()); // 强制等于传入的 ids
        summary.seq = next_seq(&mut tx, thread_id).await?; // ← 分配新 seq

        // summary.created_at 由调用方设置（= 最后一条被覆盖消息的 created_at）
        // 这里可以做防御性校验，也可以信任调用方

        // ④ 插入摘要
        insert_message(&mut tx, &summary).await?;

        // ── 提交 ────────────────────────────────────────────────────
        tx.commit().await.map_err(persistence)?;
        Ok(())
    }
}
