use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde_json::{Value, json};

use crate::error::Error;
use crate::store::{Namespace, Store};
use crate::tool::{Tool, ToolSpec};

/// A tool that saves a fact into the [`Store`] under a namespace.
///
/// Registered automatically by [`AgentBuilder::store`](crate::AgentBuilder::store).
pub struct RememberTool {
    pub(crate) store: Arc<dyn Store>,
}

/// A tool that recalls facts from the [`Store`] under a namespace prefix.
///
/// Registered automatically by [`AgentBuilder::store`](crate::AgentBuilder::store).
pub struct RecallTool {
    pub(crate) store: Arc<dyn Store>,
}

impl Tool for RememberTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "remember",
            "Save a fact or preference under a namespace for later recall.",
            json!({
                "type": "object",
                "properties": {
                    "namespace": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "The namespace path, e.g. [\"users\", \"u-42\", \"prefs\"]."
                    },
                    "key": {
                        "type": "string",
                        "description": "The fact's key, unique within the namespace."
                    },
                    "value": {
                        "description": "The JSON value to remember."
                    }
                },
                "required": ["namespace", "key", "value"]
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        let store = Arc::clone(&self.store);
        Box::pin(async move {
            let namespace = parse_namespace(&args)?;
            let key = args
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| tool_error("remember", "missing `key`"))?
                .to_string();
            let value = args
                .get("value")
                .cloned()
                .ok_or_else(|| tool_error("remember", "missing `value`"))?;

            store.put(&namespace, &key, value).await?;
            Ok(json!({ "saved": true, "namespace": namespace, "key": key }).to_string())
        })
    }
}

impl Tool for RecallTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "recall",
            "Recall saved facts under a namespace prefix.",
            json!({
                "type": "object",
                "properties": {
                    "namespace": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "The namespace prefix to search, e.g. [\"users\", \"u-42\"]."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of facts to return (default 10)."
                    }
                },
                "required": ["namespace"]
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        let store = Arc::clone(&self.store);
        Box::pin(async move {
            let namespace = parse_namespace(&args)?;
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;

            let items = store.search(&namespace, 0, limit).await?;
            Ok(json!(items).to_string())
        })
    }
}

/// Parses the `namespace` array argument into a [`Namespace`].
fn parse_namespace(args: &Value) -> Result<Namespace, Error> {
    args.get("namespace")
        .and_then(Value::as_array)
        .ok_or_else(|| tool_error("remember", "missing `namespace`"))?
        .iter()
        .map(|part| {
            part.as_str()
                .map(str::to_string)
                .ok_or_else(|| tool_error("remember", "`namespace` items must be strings"))
        })
        .collect()
}

/// Builds a [`Error::Tool`] for a malformed tool argument.
fn tool_error(name: &'static str, message: &str) -> Error {
    Error::Tool {
        name: name.to_string(),
        message: message.to_string(),
    }
}
