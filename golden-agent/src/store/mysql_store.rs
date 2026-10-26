use async_trait::async_trait;
use serde_json::Value;
use sqlx::mysql::MySqlPool;

use crate::Error;
use crate::store::ns::decode_namespace;
use crate::store::sql_common::{
    ItemRow, encode, persistence, prefix_like, row_to_item, value_json,
};
use crate::store::{Namespace, Store, StoreItem};

const DDL: &str = include_str!("mysql_store.sql");

/// A MySQL-backed [`Store`], gated on the `persistence-mysql` feature.
pub struct MySqlStore {
    pool: MySqlPool,
}

impl MySqlStore {
    /// Wraps an existing pool. The schema must already exist (call
    /// [`Self::migrate`] or run the DDL yourself).
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Opens a connection pool to `url` and wraps it.
    ///
    /// `url` is a MySQL URL like `mysql://user:password@localhost:3306/dbname`.
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let pool = MySqlPool::connect(url).await.map_err(persistence)?;
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

/// Returns the current unix time in seconds, or 0 if the clock is before epoch.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[async_trait]
impl Store for MySqlStore {
    async fn put_batch(&self, items: Vec<(Namespace, String, Value)>) -> Result<(), Error> {
        if items.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(persistence)?;
        let timestamp = now();

        for (namespace, key, value) in items {
            let ns = encode(&namespace);
            // Upsert: preserve created_at on conflict, refresh value/updated_at.
            // MySQL has no `excluded` pseudo-table; the VALUES row is addressed
            // through the `new` alias.
            sqlx::query(
                r#"
                INSERT INTO store_items (namespace, `key`, value, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?)
                AS new
                ON DUPLICATE KEY UPDATE
                    value      = new.value,
                    updated_at = new.updated_at
                "#,
            )
            .bind(&ns)
            .bind(&key)
            .bind(value_json(&value)?)
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
            SELECT namespace, `key`, value, created_at, updated_at
            FROM store_items
            WHERE namespace = ? AND `key` = ?
            "#,
        )
        .bind(encode(namespace))
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
            SELECT namespace, `key`, value, created_at, updated_at
            FROM store_items
            WHERE namespace LIKE ? ESCAPE '\\'
            ORDER BY updated_at DESC, `key` ASC
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
        sqlx::query("DELETE FROM store_items WHERE namespace = ? AND `key` = ?")
            .bind(encode(namespace))
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
            WHERE namespace LIKE ? ESCAPE '\\'
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
