use async_trait::async_trait;

use crate::{Error, StoredMessage};

#[async_trait]
pub trait Checkpointer: Send + Sync {
    /// Loads the visible context messages of a thread.
    ///
    /// Returns rows with `is_context = true`, ordered by `(created_at, seq)`
    /// ascending. Returns an empty `Vec` for an unknown thread.
    async fn load(&self, thread_id: &str) -> Result<Vec<StoredMessage>, Error>;

    /// Persists new messages for a thread.
    ///
    /// Implementations assign the real `seq` values (strictly increasing per
    /// thread) and ignore the `seq` field of each input row. The `id` field
    /// must be preserved: it is referenced by [`Self::compact`] and by
    /// [`StoredMessage::summary_of`].
    ///
    /// Appending must be atomic: either every row is written, or none.
    async fn append(&self, thread_id: &str, messages: Vec<StoredMessage>) -> Result<(), Error>;

    /// Hides a range of messages and inserts a summary covering them.
    ///
    /// Sets `is_context = false` on the rows whose `id` is in `ids`, and
    /// inserts `summary` (which must have `is_summary = true`,
    /// `is_context = true`, and `summary_of = Some(ids)`). The summary's
    /// `created_at` should equal the last covered row's `created_at` so it
    /// sorts right before the messages that follow the range.
    ///
    /// The operation is atomic. Callers must ensure the covered range does
    /// not split a tool-call pair (invariant 3).
    async fn compact(
        &self,
        thread_id: &str,
        ids: &[String],
        summary: StoredMessage,
    ) -> Result<(), Error>;
}
