use async_trait::async_trait;

use crate::Error;
use crate::checkpointer::CheckpointMessage;

#[async_trait]
pub trait Checkpointer: Send + Sync {
    /// Loads the visible context messages of a thread.
    ///
    /// Returns rows with `is_context = true`, ordered by `(created_at, seq)`
    /// ascending. Returns an empty `Vec` for an unknown thread.
    async fn load(&self, thread_id: &str) -> Result<Vec<CheckpointMessage>, Error>;

    /// Persists new messages for a thread.
    ///
    /// Implementations assign the real `seq` values (strictly increasing per
    /// thread) and ignore the `seq` field of each input row. The `id` field
    /// must be preserved: it identifies each row.
    ///
    /// Appending must be atomic: either every row is written, or none.
    async fn append(&self, thread_id: &str, messages: Vec<CheckpointMessage>) -> Result<(), Error>;
}
