#[allow(clippy::module_inception)]
mod checkpointer;
mod in_memory_checkpointer;
mod message;

#[cfg(feature = "persistence-sqlite")]
mod sqlite_checkpointer;

#[cfg(feature = "persistence-sqlite")]
pub use sqlite_checkpointer::SqliteCheckpointer;

pub use checkpointer::Checkpointer;
pub use in_memory_checkpointer::InMemoryCheckpointer;
pub use message::CheckpointMessage;
