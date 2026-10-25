mod in_memory_store;
mod memory_tools;

// The trait module is named `store` to mirror `store::Store`; allow the nesting.
#[allow(clippy::module_inception)]
mod store;

#[cfg(feature = "persistence-sqlite")]
mod sqlite_store;

#[cfg(feature = "persistence-sqlite")]
pub use sqlite_store::SqliteStore;

pub use in_memory_store::InMemoryStore;
pub(crate) use memory_tools::{RecallTool, RememberTool};
pub use store::{Namespace, Store, StoreItem};
