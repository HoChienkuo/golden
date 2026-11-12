use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};

use super::mutation::{path_for_mutation, write_text};
use super::workspace::{Workspace, error};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "write_file";

/// Creates a UTF-8 file or explicitly replaces an existing regular file.
pub struct WriteFileTool {
    workspace: Workspace,
}

impl WriteFileTool {
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
    content: String,
    #[serde(default)]
    overwrite: bool,
}

impl Tool for WriteFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Write a UTF-8 file. The parent directory must exist.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative file path." },
                    "content": { "type": "string", "description": "Complete file contents." },
                    "overwrite": { "type": "boolean", "description": "Allow replacing an existing regular file. Defaults to false." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.mutate(args, write_file)
    }
}

fn write_file(root: &Path, args: Arguments) -> Result<String, Error> {
    let path = path_for_mutation(root, &args.path, NAME)?;
    let existing = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() && args.overwrite => Some(metadata),
        Ok(_) => return Err(error(NAME, "path exists and cannot be overwritten")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(error(NAME, err)),
    };

    write_text(&path, &args.content, existing, NAME)?;
    Ok(json!({ "path": args.path, "bytes_written": args.content.len() }).to_string())
}
