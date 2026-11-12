use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};

use super::mutation::{path_for_mutation, require_missing, temporary_file};
use super::workspace::{Workspace, error};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "copy_path";

/// Copies a workspace file or recursively copies a directory.
pub struct CopyPathTool {
    workspace: Workspace,
}

impl CopyPathTool {
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

impl Tool for CopyPathTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Copy a workspace file or directory recursively. The destination must not exist.",
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
        self.workspace.mutate(args, copy_path)
    }
}

fn copy_path(root: &Path, args: Arguments) -> Result<String, Error> {
    let source = path_for_mutation(root, &args.source, NAME)?;
    let destination = path_for_mutation(root, &args.destination, NAME)?;
    if destination.starts_with(&source) {
        return Err(error(NAME, "destination must be outside the source"));
    }
    require_missing(&destination, NAME)?;
    let metadata = fs::symlink_metadata(&source).map_err(|err| error(NAME, err))?;
    if metadata.is_file() {
        copy_file(&source, &destination)?;
    } else if metadata.is_dir() {
        fs::create_dir(&destination).map_err(|err| error(NAME, err))?;
        if let Err(err) = copy_directory(source, &destination) {
            return match fs::remove_dir_all(&destination) {
                Ok(()) => Err(err),
                Err(cleanup) => Err(error(
                    NAME,
                    format!("{err}; could not remove incomplete destination: {cleanup}"),
                )),
            };
        }
    } else {
        return Err(error(NAME, "source must be a regular file or directory"));
    }
    Ok(json!({ "source": args.source, "destination": args.destination }).to_string())
}

fn copy_directory(source: PathBuf, destination: &Path) -> Result<(), Error> {
    let mut pending = vec![(source, destination.to_path_buf())];
    while let Some((source, destination)) = pending.pop() {
        for entry in fs::read_dir(source).map_err(|err| error(NAME, err))? {
            let entry = entry.map_err(|err| error(NAME, err))?;
            let kind = entry.file_type().map_err(|err| error(NAME, err))?;
            let target = destination.join(entry.file_name());
            if kind.is_dir() {
                fs::create_dir(&target).map_err(|err| error(NAME, err))?;
                pending.push((entry.path(), target));
            } else if kind.is_file() {
                copy_file(&entry.path(), &target)?;
            } else {
                return Err(error(
                    NAME,
                    format!("cannot copy {}", entry.path().display()),
                ));
            }
        }
    }
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), Error> {
    let mut source = File::open(source).map_err(|err| error(NAME, err))?;
    let mut temporary = temporary_file(destination, NAME)?;
    io::copy(&mut source, &mut temporary).map_err(|err| error(NAME, err))?;
    let metadata = source.metadata().map_err(|err| error(NAME, err))?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())
        .map_err(|err| error(NAME, err))?;
    temporary
        .persist_noclobber(destination)
        .map_err(|err| error(NAME, err.error))?;
    Ok(())
}
