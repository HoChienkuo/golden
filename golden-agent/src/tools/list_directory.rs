use std::fs;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::workspace::{Workspace, error, resolve};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "list_directory";
const MAX_ENTRIES: usize = 200;

/// Lists one level of a directory inside a configured workspace.
///
/// Register an instance with [`ToolSet::with_tool`](crate::ToolSet::with_tool).
pub struct ListDirectoryTool {
    workspace: Workspace,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    #[serde(default)]
    path: PathBuf,
}

#[derive(Serialize)]
struct Entry {
    name: String,
    kind: &'static str,
}

impl ListDirectoryTool {
    /// Creates the tool with an existing directory as its workspace root.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            workspace: Workspace::new(root.as_ref(), NAME)?,
        })
    }
}

impl Tool for ListDirectoryTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "List files and subdirectories one level deep in the workspace.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Workspace-relative directory. Defaults to the workspace root."
                    }
                },
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.run(args, list_directory)
    }
}

fn list_directory(root: &Path, args: Arguments) -> Result<String, Error> {
    let path = args.path.as_path();
    let directory = resolve(root, path, NAME)?;

    let mut entries = Vec::new();
    let mut truncated = false;
    for entry in fs::read_dir(directory).map_err(tool_error)? {
        let entry = entry.map_err(tool_error)?;
        if entries.len() == MAX_ENTRIES {
            truncated = true;
            break;
        }
        let file_type = entry.file_type().map_err(tool_error)?;
        let kind = if file_type.is_dir() {
            "directory"
        } else if file_type.is_file() {
            "file"
        } else if file_type.is_symlink() {
            "symlink"
        } else {
            "other"
        };
        entries.push(Entry {
            name: entry.file_name().to_string_lossy().into_owned(),
            kind,
        });
    }
    entries.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    Ok(json!({ "path": path, "entries": entries, "truncated": truncated }).to_string())
}

fn tool_error(message: impl std::fmt::Display) -> Error {
    error(NAME, message)
}
