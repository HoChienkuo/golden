use async_trait::async_trait;
use serde_json::Value;
use sqlx::FromRow;
use sqlx::sqlite::SqlitePool;

use crate::Error;
use crate::store::{Namespace, Store, StoreItem};

const DDL: &str = include_str!("sqlite_store.sql");

/// The separator used to flatten a [`Namespace`] into a single column value.
const SEP: char = '/';

/// A SQLite-backed [`Store`], gated on the `persistence-sqlite` feature.
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Wraps an existing pool. The schema must already exist (call
    /// [`Self::migrate`] or run the DDL yourself).
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

    /// Creates the `store_items` table and index if they do not exist.
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

/// Flattens a namespace into a length-prefixed, `/`-joined string.
fn encode_namespace(namespace: &[String]) -> String {
    namespace
        .iter()
        .map(|segment| format!("{}:{segment}", segment.len()))
        .collect::<Vec<_>>()
        .join(&SEP.to_string())
}

/// Reverses [`encode_namespace`].
fn decode_namespace(encoded: &str) -> Result<Namespace, Error> {
    if encoded.is_empty() {
        return Ok(Vec::new());
    }
    encoded
        .split(SEP)
        .map(|part| {
            let (len, rest) = part
                .split_once(':')
                .ok_or_else(|| persistence(format!("malformed namespace segment: {part}")))?;
            let len: usize = len
                .parse()
                .map_err(|_| persistence(format!("malformed namespace length: {len}")))?;
            if rest.len() != len {
                return Err(persistence(format!(
                    "namespace segment length mismatch: {part}"
                )));
            }
            Ok(rest.to_string())
        })
        .collect()
}

/// Returns the LIKE pattern matching `prefix` and any deeper namespace.
fn prefix_like(prefix: &[String]) -> String {
    if prefix.is_empty() {
        return "%".to_string();
    }
    // Escape LIKE metacharacters in the encoded prefix, then append a wildcard
    // to match any deeper namespace.
    let encoded = encode_namespace(prefix);
    let escaped = encoded
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("{escaped}%")
}

#[derive(FromRow)]
struct ItemRow {
    namespace: String,
    key: String,
    value: String,
    created_at: i64,
    updated_at: i64,
}

fn row_to_item(row: ItemRow) -> Result<StoreItem, Error> {
    Ok(StoreItem {
        namespace: decode_namespace(&row.namespace)?,
        key: row.key,
        value: serde_json::from_str(&row.value).map_err(persistence)?,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// Returns the current unix time in seconds, or 0 if the clock is before epoch.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[async_trait]
impl Store for SqliteStore {
    async fn put_batch(&self, items: Vec<(Namespace, String, Value)>) -> Result<(), Error> {
        if items.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(persistence)?;
        let timestamp = now();

        for (namespace, key, value) in items {
            let ns = encode_namespace(&namespace);
            // Upsert: preserve created_at on conflict, refresh value/updated_at.
            sqlx::query(
                r#"
                INSERT INTO store_items (namespace, key, value, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT (namespace, key) DO UPDATE SET
                    value      = excluded.value,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&ns)
            .bind(&key)
            .bind(serde_json::to_string(&value).map_err(persistence)?)
            .bind(timestamp)
            .bind(timestamp)
            .execute(&mut *tx)
            .await
            .map_err(persistence)?;
        }

        tx.commit().await.map_err(persistence)?;
        Ok(())
    }

    async fn get(&self, namespace: &Namespace, key: &str) -> Result<Option<StoreItem>, Error> {
        let row = sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT namespace, key, value, created_at, updated_at
            FROM store_items
            WHERE namespace = ? AND key = ?
            "#,
        )
        .bind(encode_namespace(namespace))
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(persistence)?;

        row.map(row_to_item).transpose()
    }

    async fn search(
        &self,
        prefix: &Namespace,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<StoreItem>, Error> {
        let rows = sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT namespace, key, value, created_at, updated_at
            FROM store_items
            WHERE namespace LIKE ? ESCAPE '\'
            ORDER BY updated_at DESC, key ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(prefix_like(prefix))
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(persistence)?;

        rows.into_iter().map(row_to_item).collect()
    }

    async fn delete(&self, namespace: &Namespace, key: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM store_items WHERE namespace = ? AND key = ?")
            .bind(encode_namespace(namespace))
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(persistence)?;
        Ok(())
    }

    async fn list_namespaces(
        &self,
        prefix: &Namespace,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Namespace>, Error> {
        let rows = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT namespace
            FROM store_items
            WHERE namespace LIKE ? ESCAPE '\'
            ORDER BY namespace ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(prefix_like(prefix))
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(persistence)?;

        rows.iter().map(|ns| decode_namespace(ns)).collect()
    }
}
