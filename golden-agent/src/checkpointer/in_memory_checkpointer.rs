use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;

use crate::Error;
use crate::checkpointer::{CheckpointMessage, Checkpointer};

#[derive(Debug, Default)]
pub struct InMemoryCheckpointer {
    threads: RwLock<HashMap<String, ThreadState>>,
}

#[derive(Debug, Default)]
struct ThreadState {
    messages: Vec<CheckpointMessage>,
    next_seq: u64,
}

impl InMemoryCheckpointer {
    /// Creates an empty checkpointer.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Checkpointer for InMemoryCheckpointer {
    async fn load(&self, thread_id: &str) -> Result<Vec<CheckpointMessage>, Error> {
        // The lock is only ever held across synchronous work, never an await.
        // A poisoned lock still holds valid data (nothing here can leave it
        // inconsistent), so recover the guard instead of propagating the panic.
        let threads = self
            .threads
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(state) = threads.get(thread_id) else {
            return Ok(Vec::new());
        };
        let mut visible: Vec<CheckpointMessage> = state
            .messages
            .iter()
            .filter(|m| m.is_context)
            .cloned()
            .collect();
        visible.sort_by_key(|a| (a.created_at, a.seq));
        Ok(visible)
    }

    async fn append(&self, thread_id: &str, messages: Vec<CheckpointMessage>) -> Result<(), Error> {
        let mut threads = self
            .threads
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = threads.entry(thread_id.to_string()).or_default();
        for mut message in messages {
            message.thread_id = thread_id.to_string();
            message.seq = state.next_seq;
            state.next_seq += 1;
            state.messages.push(message);
        }
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
        let mut threads = self
            .threads
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(state) = threads.get_mut(thread_id) else {
            return Ok(());
        };
        for message in &mut state.messages {
            if ids.contains(&message.id) {
                message.is_context = is_context;
            }
        }
        Ok(())
    }
}
