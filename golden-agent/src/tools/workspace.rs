use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use ignore::{Walk, WalkBuilder};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Error;

static MUTATIONS: Mutex<()> = Mutex::new(());

pub(super) struct Workspace {
    root: Arc<Path>,
    tool: &'static str,
}

impl Workspace {
    pub(super) fn new(root: &Path, tool: &'static str) -> Result<Self, Error> {
        let root = root.canonicalize().map_err(|err| error(tool, err))?;
        if !root.is_dir() {
            return Err(error(tool, "workspace root must be a directory"));
        }
        Ok(Self {
            root: root.into(),
            tool,
        })
    }

    pub(super) fn run<A: DeserializeOwned + Send + 'static>(
        &self,
        args: Value,
        operation: fn(&Path, A) -> Result<String, Error>,
    ) -> BoxFuture<'static, Result<String, Error>> {
        self.run_operation(args, operation)
    }

    pub(super) fn mutate<A: DeserializeOwned + Send + 'static>(
        &self,
        args: Value,
        operation: fn(&Path, A) -> Result<String, Error>,
    ) -> BoxFuture<'static, Result<String, Error>> {
        let tool = self.tool;
        self.run_operation(args, move |root, args| {
            let _guard = MUTATIONS.lock().map_err(|err| error(tool, err))?;
            operation(root, args)
        })
    }

    fn run_operation<A: DeserializeOwned + Send + 'static>(
        &self,
        args: Value,
        operation: impl FnOnce(&Path, A) -> Result<String, Error> + Send + 'static,
    ) -> BoxFuture<'static, Result<String, Error>> {
        let root = Arc::clone(&self.root);
        let tool = self.tool;
        Box::pin(async move {
            if !args.is_object() {
                return Err(error(tool, "arguments must be a JSON object"));
            }
            let args = serde_json::from_value(args).map_err(|err| error(tool, err))?;
            tokio::task::spawn_blocking(move || operation(&root, args))
                .await
                .map_err(|err| error(tool, err))?
        })
    }
}

pub(super) fn resolve(root: &Path, path: &Path, tool: &'static str) -> Result<PathBuf, Error> {
    validate_relative(path, tool)?;
    let resolved = root
        .join(path)
        .canonicalize()
        .map_err(|err| error(tool, err))?;
    if !resolved.starts_with(root) {
        return Err(error(tool, "path resolves outside the workspace"));
    }
    Ok(resolved)
}

pub(super) fn validate_relative(path: &Path, tool: &str) -> Result<(), Error> {
    if path
        .components()
        .any(|component| !matches!(component, Component::CurDir | Component::Normal(_)))
    {
        return Err(error(
            tool,
            "path must be relative to the workspace without '..'",
        ));
    }
    Ok(())
}

pub(super) fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path is inside the workspace")
        .to_string_lossy()
        .into_owned()
}

pub(super) fn walk(path: &Path) -> Walk {
    WalkBuilder::new(path)
        .require_git(false)
        .git_global(false)
        .follow_links(false)
        .sort_by_file_path(Path::cmp)
        .build()
}

pub(super) fn error(tool: &str, message: impl std::fmt::Display) -> Error {
    Error::Tool {
        name: tool.to_owned(),
        message: message.to_string(),
    }
}
