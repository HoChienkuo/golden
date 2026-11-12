use std::fs;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};

use super::mutation::{path_for_mutation, write_text};
use super::workspace::{Workspace, error};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "edit_file";

/// Replaces a unique occurrence of text in an existing UTF-8 file.
pub struct EditFileTool {
    workspace: Workspace,
}

impl EditFileTool {
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
    old_text: String,
    new_text: String,
}

impl Tool for EditFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Replace exactly one occurrence of text in an existing UTF-8 file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative file path." },
                    "old_text": { "type": "string", "minLength": 1, "description": "Exact text to replace, including enough context for a unique match." },
                    "new_text": { "type": "string", "description": "Replacement text. Use an empty string to remove the match." }
                },
                "required": ["path", "old_text", "new_text"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.mutate(args, edit_file)
    }
}

fn edit_file(root: &Path, args: Arguments) -> Result<String, Error> {
    if args.old_text.is_empty() {
        return Err(error(NAME, "old_text must not be empty"));
    }
    let path = path_for_mutation(root, &args.path, NAME)?;
    let metadata = fs::symlink_metadata(&path).map_err(|err| error(NAME, err))?;
    if !metadata.is_file() {
        return Err(error(NAME, "path must be a regular file"));
    }
    let mut content = fs::read_to_string(&path).map_err(|err| error(NAME, err))?;
    let Some(start) = content.find(&args.old_text) else {
        return Err(error(NAME, "old_text was not found"));
    };
    let end = start + args.old_text.len();
    if content.rfind(&args.old_text) != Some(start) {
        return Err(error(NAME, "old_text matches more than once"));
    }
    content.replace_range(start..end, &args.new_text);
    write_text(&path, &content, Some(metadata), NAME)?;
    Ok(json!({ "path": args.path, "replacements": 1 }).to_string())
}
