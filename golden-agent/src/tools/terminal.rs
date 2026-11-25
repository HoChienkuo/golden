use std::path::{Path, PathBuf};
use std::process::Stdio;

use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use super::workspace::{Workspace, error, validate_relative};
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "terminal";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 300_000;
const MAX_OUTPUT_BYTES: usize = 16 * 1024;

/// Runs a shell command with the workspace directory as its working directory.
pub struct TerminalTool {
    workspace: Workspace,
}

impl TerminalTool {
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
    command: String,
    #[serde(default)]
    cwd: PathBuf,
    timeout_ms: Option<u64>,
}

impl Tool for TerminalTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Run a shell command in the workspace and return merged stdout and stderr. The process is killed when it times out.",
            json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to run." },
                    "cwd": { "type": "string", "description": "Workspace-relative working directory. Defaults to the workspace root." },
                    "timeout_ms": { "type": "integer", "minimum": 1, "description": "Maximum runtime in milliseconds. Defaults to 30000." }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        let root = self.workspace.root();
        let tool = self.workspace.tool();
        Box::pin(async move {
            if !args.is_object() {
                return Err(error(tool, "arguments must be a JSON object"));
            }
            let args = serde_json::from_value(args).map_err(|err| error(tool, err))?;
            run(root.as_ref(), args).await
        })
    }
}

async fn run(root: &Path, args: Arguments) -> Result<String, Error> {
    if args.command.trim().is_empty() {
        return Err(error(NAME, "command must not be empty"));
    }
    let timeout_ms = args.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(error(
            NAME,
            format!("timeout_ms must be at most {MAX_TIMEOUT_MS}"),
        ));
    }

    validate_relative(&args.cwd, NAME).map_err(|err| error(NAME, err))?;
    let cwd = root.join(&args.cwd);
    if !cwd.is_dir() {
        return Err(error(NAME, "cwd must be a directory inside the workspace"));
    }

    let (program, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let mut child = Command::new(program)
        .arg(flag)
        .arg(&args.command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|err| error(NAME, err))?;

    let mut stdout = child
        .stdout
        .take()
        .map(|pipe| pipe.take(MAX_OUTPUT_BYTES as u64 + 1))
        .ok_or_else(|| error(NAME, "stdout pipe was not available"))?;
    let mut stderr = child
        .stderr
        .take()
        .map(|pipe| pipe.take(MAX_OUTPUT_BYTES as u64 + 1))
        .ok_or_else(|| error(NAME, "stderr pipe was not available"))?;
    let left = async {
        let mut buffer = Vec::new();
        stdout
            .read_to_end(&mut buffer)
            .await
            .map_err(|err| error(NAME, err))?;
        Ok::<_, Error>(buffer)
    };
    let right = async {
        let mut buffer = Vec::new();
        stderr
            .read_to_end(&mut buffer)
            .await
            .map_err(|err| error(NAME, err))?;
        Ok::<_, Error>(buffer)
    };
    let reader = async move {
        // Read both pipes concurrently so a command that fills one pipe's
        // buffer cannot block while we drain the other.
        let (left, right) = tokio::join!(left, right);
        Ok::<_, Error>((left?, right?))
    };

    let outcome = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), reader).await;
    let (stdout_bytes, stderr_bytes) = match outcome {
        Ok(reads) => reads?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error(
                NAME,
                format!("command timed out after {timeout_ms} ms and was killed"),
            ));
        }
    };

    let status = child.wait().await.map_err(|err| error(NAME, err))?;
    let (output, truncated) = merge_output(&stdout_bytes, &stderr_bytes);
    Ok(json!({
        "command": args.command,
        "exit_code": status.code(),
        "output": output,
        "truncated": truncated
    })
    .to_string())
}

/// Merges stdout and stderr, then truncates the combined text to
/// [`MAX_OUTPUT_BYTES`] at a character boundary. The byte limit is measured on
/// the merged text so that it also bounds non-UTF-8 output.
fn merge_output(stdout: &[u8], stderr: &[u8]) -> (String, bool) {
    let mut output = String::new();
    for bytes in [stdout, stderr] {
        if bytes.is_empty() {
            continue;
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&String::from_utf8_lossy(bytes));
    }

    let total_bytes = stdout.len() + stderr.len();
    let truncated = output.len() > MAX_OUTPUT_BYTES;
    if !truncated {
        return (output, false);
    }
    let end = output
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= MAX_OUTPUT_BYTES)
        .last()
        .unwrap_or(0);
    output.truncate(end);
    let lines = output.lines().count();
    output.push_str(&format!(
        "\n[output truncated to {MAX_OUTPUT_BYTES} of {total_bytes} bytes; showing the first {lines} lines]"
    ));
    (output, true)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Tool;

    fn workspace() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("golden-terminal-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&path).unwrap();
        path
    }

    async fn call(root: &Path, args: Value) -> Value {
        serde_json::from_str(&TerminalTool::new(root).unwrap().call(args).await.unwrap()).unwrap()
    }

    #[tokio::test]
    async fn terminal_runs_command_in_the_workspace() {
        let root = workspace();
        std::fs::create_dir(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/sample.txt"), "hello\n").unwrap();

        let result = call(&root, json!({ "command": "echo hello && pwd" })).await;
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["truncated"], false);
        let output = result["output"].as_str().unwrap();
        assert!(output.contains("hello"));
        assert!(output.contains(&root.to_string_lossy().into_owned()));

        let result = call(&root, json!({ "command": "echo to-stderr 1>&2" })).await;
        assert_eq!(result["exit_code"], 0);
        assert!(result["output"].as_str().unwrap().contains("to-stderr"));

        let result = call(
            &root,
            json!({ "command": "pwd && cat sample.txt", "cwd": "sub" }),
        )
        .await;
        assert_eq!(result["exit_code"], 0);
        assert!(result["output"].as_str().unwrap().contains("hello"));

        let result = call(&root, json!({ "command": "exit 3" })).await;
        assert_eq!(result["exit_code"], 3);
    }

    #[tokio::test]
    async fn terminal_kills_timeouts_and_truncates_large_output() {
        let root = workspace();
        let error = call_err(&root, json!({ "command": "sleep 5", "timeout_ms": 200 })).await;
        assert!(error.contains("timed out after 200 ms"));

        let result = call(
            &root,
            json!({ "command": "for i in $(seq 1 3000); do echo line-$i-payload; done" }),
        )
        .await;
        assert_eq!(result["truncated"], true);
        let output = result["output"].as_str().unwrap();
        assert!(output.len() <= MAX_OUTPUT_BYTES + 128);
        assert!(output.contains("line-1-payload\n"));
        assert!(!output.contains("line-3000"));
    }

    #[tokio::test]
    async fn terminal_rejects_bad_arguments() {
        let root = workspace();
        for args in [
            json!({ "command": "echo hi", "cwd": "../outside" }),
            json!({ "command": "" }),
            json!({ "command": "echo hi", "timeout_ms": MAX_TIMEOUT_MS + 1 }),
        ] {
            let message = call_err(&root, args).await;
            assert!(!message.is_empty());
        }
    }

    async fn call_err(root: &Path, args: Value) -> String {
        match TerminalTool::new(root).unwrap().call(args).await {
            Err(Error::Tool { message, .. }) => message,
            other => panic!("expected a tool error, got {other:?}"),
        }
    }
}
