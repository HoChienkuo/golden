use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::workspace::{Workspace, error, relative_path, resolve, walk};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "grep";
const MAX_RESULTS: usize = 100;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_LINE_CHARS: usize = 500;

/// Searches workspace text files by regular expression.
pub struct GrepTool {
    workspace: Workspace,
}

impl GrepTool {
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

#[derive(Serialize)]
struct Match {
    path: String,
    line: usize,
    text: String,
    truncated: bool,
}

impl Tool for GrepTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Search UTF-8 text files for a regex, with 1-based line numbers. Skips hidden, ignored, binary, and large files.",
            json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regular expression to match against each line." },
                    "path": { "type": "string", "description": "Workspace-relative file or directory. Defaults to the root." }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.run(args, grep)
    }
}

fn grep(root: &Path, args: Arguments) -> Result<String, Error> {
    let path = resolve(root, &args.path, NAME)?;
    if !path.is_file() && !path.is_dir() {
        return Err(error(NAME, "search path must be a file or directory"));
    }
    if args.pattern.is_empty() || args.pattern.len() > 4096 {
        return Err(error(NAME, "pattern must contain 1 to 4096 bytes"));
    }
    let regex = Regex::new(&args.pattern).map_err(|err| error(NAME, err))?;

    let mut matches = Vec::new();
    let mut skipped_files = 0;
    for entry in walk(&path) {
        let entry = entry.map_err(|err| error(NAME, err))?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Some(text) = read_text(entry.path())? else {
            skipped_files += 1;
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let Some(found) = regex.find(line) else {
                continue;
            };
            if matches.len() == MAX_RESULTS {
                return Ok(json!({ "matches": matches, "skipped_files": skipped_files, "truncated": true }).to_string());
            }
            let start = match line.char_indices().nth(MAX_LINE_CHARS) {
                Some((end, _)) if found.end() > end => line[..found.start()]
                    .char_indices()
                    .rev()
                    .nth(MAX_LINE_CHARS / 4 - 1)
                    .map_or(0, |(index, _)| index),
                _ => 0,
            };
            let excerpt = &line[start..];
            let end = excerpt
                .char_indices()
                .nth(MAX_LINE_CHARS)
                .map(|(index, _)| index);
            matches.push(Match {
                path: relative_path(root, entry.path()),
                line: index + 1,
                text: excerpt[..end.unwrap_or(excerpt.len())].to_owned(),
                truncated: start > 0 || end.is_some(),
            });
        }
    }
    Ok(
        json!({ "matches": matches, "skipped_files": skipped_files, "truncated": false })
            .to_string(),
    )
}

fn read_text(path: &Path) -> Result<Option<String>, Error> {
    let file = File::open(path).map_err(|err| error(NAME, err))?;
    if file.metadata().map_err(|err| error(NAME, err))?.len() > MAX_FILE_BYTES {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| error(NAME, err))?;
    if bytes.len() as u64 > MAX_FILE_BYTES || bytes.contains(&0) {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok())
}
