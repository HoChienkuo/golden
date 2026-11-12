use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};

use super::workspace::{Workspace, error, resolve};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "read_file";
const MAX_LINES: usize = 200;
const MAX_BYTES: usize = 64 * 1024;

/// Reads a bounded range of lines from a UTF-8 workspace file.
pub struct ReadFileTool {
    workspace: Workspace,
}

impl ReadFileTool {
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
    start_line: Option<usize>,
    end_line: Option<usize>,
}

impl Tool for ReadFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Read a range of lines from a UTF-8 file in the workspace. Returns up to 200 lines.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative file path." },
                    "start_line": { "type": "integer", "minimum": 1, "description": "First line to read. Defaults to 1." },
                    "end_line": { "type": "integer", "minimum": 1, "description": "Last line to read, inclusive. Defaults to the end of the file." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        self.workspace.run(args, read_file)
    }
}

fn read_file(root: &Path, args: Arguments) -> Result<String, Error> {
    let start = args.start_line.unwrap_or(1);
    let end = args.end_line.unwrap_or(usize::MAX);
    if start == 0 || end < start {
        return Err(error(
            NAME,
            "line numbers must satisfy 1 <= start_line <= end_line",
        ));
    }
    let path = resolve(root, &args.path, NAME)?;
    if !path.is_file() {
        return Err(error(NAME, "path must be a regular file"));
    }
    let mut reader = BufReader::new(File::open(path).map_err(|err| error(NAME, err))?);
    let mut content = String::new();
    let mut buffer = Vec::new();
    let mut last_line = None;
    let mut truncated = false;
    for number in 1..=end {
        if number - 1 == start.saturating_add(MAX_LINES - 1) {
            truncated = !reader
                .fill_buf()
                .map_err(|err| error(NAME, err))?
                .is_empty();
            break;
        }

        buffer.clear();
        let length = reader
            .by_ref()
            .take((MAX_BYTES + 1) as u64)
            .read_until(b'\n', &mut buffer)
            .map_err(|err| error(NAME, err))?;
        if length == 0 {
            break;
        }
        if length > MAX_BYTES {
            if number < start {
                if !buffer.ends_with(b"\n") {
                    reader.skip_until(b'\n').map_err(|err| error(NAME, err))?;
                }
                continue;
            }
            if content.is_empty() {
                return Err(error(
                    NAME,
                    format!("line {number} exceeds {MAX_BYTES} bytes"),
                ));
            }
            truncated = true;
            break;
        }
        if number < start {
            continue;
        }
        if content.len() + length > MAX_BYTES {
            truncated = true;
            break;
        }
        if buffer.contains(&0) {
            return Err(error(NAME, "file contains binary data"));
        }
        content.push_str(std::str::from_utf8(&buffer).map_err(|err| error(NAME, err))?);
        last_line = Some(number);
    }

    Ok(json!({
        "path": args.path,
        "start_line": start,
        "end_line": last_line,
        "content": content,
        "truncated": truncated
    })
    .to_string())
}
