use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Error;

/// A namespace path: `("users", "u-42", "preferences")`.
pub type Namespace = Vec<String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreItem {
    /// The namespace this item belongs to.
    pub namespace: Namespace,

    /// The key, unique within [`Self::namespace`].
    pub key: String,

    /// The stored value.
    pub value: Value,

    /// The creation timestamp (unix seconds), assigned on first write.
    pub created_at: i64,

    /// The last update timestamp (unix seconds), assigned on every write.
    /// This is the primary sort key of [`Store::search`].
    pub updated_at: i64,
}

/// A long-term, cross-thread key-value store.
///
/// Unlike [`Checkpointer`](crate::Checkpointer), which persists the linear
/// message history of a single thread, a `Store` holds durable facts keyed by
/// a hierarchical [`Namespace`] and a `key`.
#[async_trait]
pub trait Store: Send + Sync {
    /// Upserts one item and assigns its timestamps.
    ///
    /// Creates the namespace implicitly. The `created_at` of an existing item
    /// is preserved; `updated_at` is refreshed.
    async fn put(&self, namespace: &Namespace, key: &str, value: Value) -> Result<(), Error> {
        self.put_batch(vec![(namespace.clone(), key.to_string(), value)])
            .await
    }

    /// Upserts many items atomically.
    ///
    /// Either every item is written, or none. When the same `(namespace, key)`
    /// appears more than once in the batch, the last occurrence wins.
    async fn put_batch(&self, items: Vec<(Namespace, String, Value)>) -> Result<(), Error>;

    /// Returns one item, or `None`.
    async fn get(&self, namespace: &Namespace, key: &str) -> Result<Option<StoreItem>, Error>;

    /// Lists items under a namespace prefix, ordered by `updated_at`.
    async fn search(
        &self,
        prefix: &Namespace,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<StoreItem>, Error>;

    /// Deletes one item. Deleting a missing key is not an error.
    async fn delete(&self, namespace: &Namespace, key: &str) -> Result<(), Error>;

    /// Lists the distinct namespaces under a prefix, ordered lexically.
    ///
    /// `prefix` has the same segment-wise meaning as in [`search`](Self::search).
    /// Returns the full namespace of each matching item, deduplicated.
    async fn list_namespaces(
        &self,
        prefix: &Namespace,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Namespace>, Error>;
}
