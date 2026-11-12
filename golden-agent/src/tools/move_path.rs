use std::fs;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};

use super::mutation::{path_for_mutation, require_missing};
use super::workspace::{Workspace, error};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "move_path";

/// Renames or moves a workspace file or directory to a new path.
pub struct MovePathTool {
    workspace: Workspace,
}

impl MovePathTool {
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
    source: PathBuf,
    destination: PathBuf,
}

impl Tool for MovePathTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Move or rename a workspace file or directory. The destination must not exist.",
            json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Workspace-relative source path." },
                    "destination": { "type": "string", "description": "Workspace-relative destination. Its parent must exist." }
                },
                "required": ["source", "destination"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.mutate(args, move_path)
    }
}

fn move_path(root: &Path, args: Arguments) -> Result<String, Error> {
    let source = path_for_mutation(root, &args.source, NAME)?;
    let destination = path_for_mutation(root, &args.destination, NAME)?;
    let metadata = fs::symlink_metadata(&source).map_err(|err| error(NAME, err))?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(error(NAME, "source must be a regular file or directory"));
    }
    if destination.starts_with(&source) {
        return Err(error(NAME, "destination must be outside the source"));
    }
    require_missing(&destination, NAME)?;
    fs::rename(source, destination).map_err(|err| error(NAME, err))?;
    Ok(json!({ "source": args.source, "destination": args.destination }).to_string())
}
