use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use globset::GlobBuilder;
use serde::Deserialize;
use serde_json::{Value, json};

use super::workspace::{Workspace, error, relative_path, resolve, walk};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "find_path";
const MAX_RESULTS: usize = 200;

/// Finds workspace paths that match a glob.
pub struct FindPathTool {
    workspace: Workspace,
}

impl FindPathTool {
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
    pattern: String,
    #[serde(default)]
    path: PathBuf,
}

impl Tool for FindPathTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Find paths by glob, for example '**/*.rs'. Skips hidden and ignored entries.",
            json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Glob relative to the search directory." },
                    "path": { "type": "string", "description": "Workspace-relative search directory. Defaults to the root." }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.run(args, find_path)
    }
}

fn find_path(root: &Path, args: Arguments) -> Result<String, Error> {
    let directory = resolve(root, &args.path, NAME)?;
    if !directory.is_dir() {
        return Err(error(NAME, "search path must be a directory"));
    }
    if args.pattern.is_empty() || args.pattern.len() > 4096 {
        return Err(error(NAME, "glob must contain 1 to 4096 bytes"));
    }
    let matcher = GlobBuilder::new(&args.pattern)
        .literal_separator(true)
        .backslash_escape(true)
        .build()
        .map_err(|err| error(NAME, err))?
        .compile_matcher();

    let mut paths = Vec::new();
    for entry in walk(&directory) {
        let entry = entry.map_err(|err| error(NAME, err))?;
        if entry.depth() == 0
            || !matcher.is_match(entry.path().strip_prefix(&directory).expect("walked path"))
        {
            continue;
        }
        if paths.len() == MAX_RESULTS {
            return Ok(json!({ "paths": paths, "truncated": true }).to_string());
        }
        paths.push(relative_path(root, entry.path()));
    }
    Ok(json!({ "paths": paths, "truncated": false }).to_string())
}
