#[allow(clippy::module_inception)]
mod checkpointer;
mod in_memory_checkpointer;
mod message;

#[cfg(feature = "persistence-mysql")]
mod mysql_checkpointer;

#[cfg(feature = "persistence-postgres")]
mod postgres_checkpointer;

#[cfg(feature = "persistence-sqlite")]
mod sqlite_checkpointer;

#[cfg(any(
    feature = "persistence-sqlite",
    feature = "persistence-mysql",
    feature = "persistence-postgres"
))]
mod sql_common;

#[cfg(feature = "persistence-mysql")]
pub use mysql_checkpointer::MySqlCheckpointer;

#[cfg(feature = "persistence-postgres")]
pub use postgres_checkpointer::PostgresCheckpointer;

#[cfg(feature = "persistence-sqlite")]
pub use sqlite_checkpointer::SqliteCheckpointer;

pub use checkpointer::Checkpointer;
pub use in_memory_checkpointer::InMemoryCheckpointer;
pub use message::CheckpointMessage;
