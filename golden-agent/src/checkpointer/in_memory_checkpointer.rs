use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;

use crate::Error;
use crate::checkpointer::{CheckpointMessage, Checkpointer};

#[derive(Debug, Default)]
pub struct InMemoryCheckpointer {
    threads: Mutex<HashMap<String, ThreadState>>,
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
        let threads = self.threads.lock().expect("mutex poisoned");
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
        let mut threads = self.threads.lock().expect("mutex posioned");
        let state = threads.entry(thread_id.to_string()).or_default();
        for mut message in messages {
            message.thread_id = thread_id.to_string();
            message.seq = state.next_seq;
            state.next_seq += 1;
            state.messages.push(message);
        }
        Ok(())
    }
}
