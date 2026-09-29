use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Per-run, read-only inputs supplied by the caller.
///
/// Values are keyed by their type: store a `CurrentUser` and a middleware reads
/// it back with [`Context::get::<CurrentUser>`](Context::get). Nothing in the
/// context reaches the model unless a middleware renders it into a message.
///
/// ```
/// use golden_agent::Context;
///
/// struct CurrentUser {
///     id: String,
/// }
///
/// let mut context = Context::new();
/// context.insert(CurrentUser {
///     id: "u-42".to_string(),
/// });
///
/// assert_eq!(context.get::<CurrentUser>().unwrap().id, "u-42");
/// ```
#[derive(Clone, Default)]
pub struct Context {
    entries: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Context {
    /// Creates an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a value, replacing any previous value of the same type.
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> &mut Self {
        self.entries.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    /// Returns the stored value of the given type, if any.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.entries.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Returns whether a value of the given type is stored.
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.entries.contains_key(&TypeId::of::<T>())
    }

    /// Returns whether the context is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Context")
            .field("entries", &self.entries.len())
            .finish()
    }
}
