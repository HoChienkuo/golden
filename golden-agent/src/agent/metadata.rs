//! Runtime metadata carried through [`Context`](crate::agent::Context).

/// The thread identifier, used as the checkpointer key.
///
/// One thread = one conversation. Insert it into [`Context`] before invoking
/// an agent that has a checkpointer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThreadId(pub String);

/// The run identifier, marking one agent turn (one `invoke` / `stream`).
///
/// Generated automatically if not supplied. Used to attribute messages to the
/// turn that produced them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunId(pub String);
