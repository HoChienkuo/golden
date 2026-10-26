mod in_memory_store;
mod memory_tools;

#[cfg(feature = "persistence-mysql")]
mod mysql_store;

#[cfg(feature = "persistence-postgres")]
mod postgres_store;

// The namespace codec is shared by every SQL-backed store.
#[cfg(any(
    feature = "persistence-sqlite",
    feature = "persistence-mysql",
    feature = "persistence-postgres"
))]
mod ns;

#[cfg(any(
    feature = "persistence-sqlite",
    feature = "persistence-mysql",
    feature = "persistence-postgres"
))]
mod sql_common;

#[cfg(feature = "persistence-sqlite")]
mod sqlite_store;

// The trait module is named `store` to mirror `store::Store`; allow the nesting.
#[allow(clippy::module_inception)]
mod store;

#[cfg(feature = "persistence-mysql")]
pub use mysql_store::MySqlStore;

#[cfg(feature = "persistence-postgres")]
pub use postgres_store::PostgresStore;

#[cfg(feature = "persistence-sqlite")]
pub use sqlite_store::SqliteStore;

pub use in_memory_store::InMemoryStore;
pub(crate) use memory_tools::{RecallTool, RememberTool};
pub use store::{Namespace, Store, StoreItem};
