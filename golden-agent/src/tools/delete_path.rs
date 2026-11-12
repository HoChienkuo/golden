use std::fs;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};

use super::mutation::path_for_mutation;
use super::workspace::{Workspace, error};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "delete_path";

/// Deletes a workspace file or directory.
pub struct DeletePathTool {
    workspace: Workspace,
}

impl DeletePathTool {
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
    #[serde(default)]
    recursive: bool,
}

impl Tool for DeletePathTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Delete a workspace file or directory. Non-empty directories require recursive: true.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative path to delete." },
                    "recursive": { "type": "boolean", "description": "Delete a directory and its contents. Defaults to false." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.mutate(args, delete_path)
    }
}

fn delete_path(root: &Path, args: Arguments) -> Result<String, Error> {
    let path = path_for_mutation(root, &args.path, NAME)?;
    let metadata = fs::symlink_metadata(&path).map_err(|err| error(NAME, err))?;
    let result = if metadata.is_file() {
        fs::remove_file(path)
    } else if metadata.is_dir() && args.recursive {
        fs::remove_dir_all(path)
    } else if metadata.is_dir() {
        fs::remove_dir(path)
    } else {
        return Err(error(NAME, "path must be a regular file or directory"));
    };
    result.map_err(|err| error(NAME, err))?;
    Ok(json!({ "path": args.path, "deleted": true }).to_string())
}
