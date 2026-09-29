use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::Error;
use crate::tool::{Tool, ToolSpec, registered_tools};

/// A name-indexed collection of the tools an agent may call.
///
/// Selection is always by a tool's registered name — the wire name the model
/// requests — never by the Rust function name. Build one with
/// [`ToolSet::registered`], narrow it with [`ToolSet::only`], and add runtime
/// tools with [`ToolSet::with_tool`].
///
/// ```no_run
/// use golden_agent::ToolSet;
///
/// // Narrow the registered tools down to a subset, by their wire names.
/// let tools = ToolSet::registered()?.only(["get_weather", "calculate"])?;
/// # Ok::<(), golden_agent::Error>(())
/// ```
#[derive(Clone, Default)]
pub struct ToolSet {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolSet {
    /// Creates an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Collects every tool registered with [`#[tool]`](macro@crate::tool).
    ///
    /// Fails with [`Error::DuplicateTool`] if two registered tools share a name,
    /// so a collision surfaces at startup instead of silently dropping a tool.
    pub fn registered() -> Result<Self, Error> {
        let mut set = Self::new();
        for definition in registered_tools()? {
            set.tools
                .insert(definition.name.to_string(), Arc::new(definition));
        }
        Ok(set)
    }

    /// Keeps only the tools with the given registered names.
    ///
    /// Fails with [`Error::UnknownTool`] if any name is not in the set, so a
    /// typo surfaces instead of silently dropping the tool.
    pub fn only<I, S>(mut self, names: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let names: Vec<String> = names
            .into_iter()
            .map(|name| name.as_ref().to_string())
            .collect();

        for name in &names {
            if !self.tools.contains_key(name) {
                return Err(Error::UnknownTool { name: name.clone() });
            }
        }

        self.tools.retain(|name, _| names.contains(name));
        Ok(self)
    }

    /// Adds a tool, replacing any tool with the same name.
    pub fn with_tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools
            .insert(tool.spec().name.to_string(), Arc::new(tool));
        self
    }

    /// Returns the tool with the given name, if present.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Returns the neutral specs of every tool, for sending to the model.
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|tool| tool.spec()).collect()
    }

    /// Returns the number of tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
