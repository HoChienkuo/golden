use std::fs;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};

use super::mutation::path_for_mutation;
use super::workspace::{Workspace, error};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "create_directory";

/// Creates a directory and its missing parents in the workspace.
pub struct CreateDirectoryTool {
    workspace: Workspace,
}

impl CreateDirectoryTool {
    /// Creates the tool with an existing directory as its workspace root.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            workspace: Workspace::new(root.as_ref(), NAME)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    path: PathBuf,
}

impl Tool for CreateDirectoryTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Create a workspace directory and any missing parent directories.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative directory path." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.mutate(args, create_directory)
    }
}

fn create_directory(root: &Path, args: Arguments) -> Result<String, Error> {
    let path = path_for_mutation(root, &args.path, NAME)?;
    fs::create_dir_all(path).map_err(|err| error(NAME, err))?;
    Ok(json!({ "path": args.path }).to_string())
}
