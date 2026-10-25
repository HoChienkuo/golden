use std::{
    collections::{BTreeSet, HashMap},
    sync::Mutex,
};

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    Error,
    store::{Namespace, Store, StoreItem},
};

/// An in-process [`Store`] backed by a `HashMap`.
///
/// All items live in memory and are dropped when the value is dropped. Useful
/// for tests, prototypes, and single-process agents.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    items: Mutex<HashMap<(Namespace, String), StoreItem>>,
}

impl InMemoryStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Returns the current unix time in seconds, or 0 if the clock is before epoch.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Returns whether `namespace` is equal to `prefix` or starts with it,
/// comparing whole path segments.
fn matches_prefix(namespace: &[String], prefix: &[String]) -> bool {
    namespace.len() >= prefix.len() && namespace[..prefix.len()] == *prefix
}

/// Sorts items newest-first, ties broken by key ascending.
fn sort_newest_first(items: &mut [StoreItem]) {
    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.key.cmp(&b.key)));
}

#[async_trait]
impl Store for InMemoryStore {
    async fn put_batch(&self, items: Vec<(Namespace, String, Value)>) -> Result<(), Error> {
        let mut map = self.items.lock().expect("mutex poisoned");
        let timestamp = now();
        for (namespace, key, value) in items {
            let entry = map
                .entry((namespace.clone(), key.clone()))
                .or_insert_with(|| StoreItem {
                    namespace,
                    key,
                    value: Value::Null,
                    created_at: timestamp,
                    updated_at: timestamp,
                });
            entry.value = value;
            entry.updated_at = timestamp;
        }
        Ok(())
    }

    async fn get(&self, namespace: &Namespace, key: &str) -> Result<Option<StoreItem>, Error> {
        let map = self.items.lock().expect("mutex poisoned");
        Ok(map.get(&(namespace.clone(), key.to_string())).cloned())
    }

    async fn search(
        &self,
        prefix: &Namespace,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<StoreItem>, Error> {
        let map = self.items.lock().expect("mutex poisoned");
        let mut matched: Vec<StoreItem> = map
            .values()
            .filter(|item| matches_prefix(&item.namespace, prefix))
            .cloned()
            .collect();
        sort_newest_first(&mut matched);
        Ok(matched.into_iter().skip(offset).take(limit).collect())
    }

    async fn delete(&self, namespace: &Namespace, key: &str) -> Result<(), Error> {
        let mut map = self.items.lock().expect("mutex poisoned");
        map.remove(&(namespace.clone(), key.to_string()));
        Ok(())
    }

    async fn list_namespaces(
        &self,
        prefix: &Namespace,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Namespace>, Error> {
        let map = self.items.lock().expect("mutex poisoned");
        let set: BTreeSet<Namespace> = map
            .values()
            .filter(|item| matches_prefix(&item.namespace, prefix))
            .map(|item| item.namespace.clone())
            .collect();
        Ok(set.into_iter().skip(offset).take(limit).collect())
    }
}
