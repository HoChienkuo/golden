use std::fs::{self, Metadata};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use tempfile::NamedTempFile;

use super::workspace::{error, resolve, validate_relative};
use crate::Error;

/// Resolves the existing prefix of a mutation path, rejecting symbolic links.
pub(super) fn path_for_mutation(
    root: &Path,
    path: &Path,
    tool: &'static str,
) -> Result<PathBuf, Error> {
    validate_relative(path, tool)?;
    let mut parts = path.components().filter_map(|part| match part {
        Component::Normal(name) => Some(name),
        _ => None,
    });
    let Some(first) = parts.next() else {
        return Err(error(
            tool,
            "path must name an entry below the workspace root",
        ));
    };

    let mut relative = PathBuf::new();
    let mut resolved = resolve(root, Path::new("."), tool)?;
    let mut missing = false;
    for name in std::iter::once(first).chain(parts) {
        #[cfg(windows)]
        {
            let name = name.to_string_lossy();
            if name.contains(':') || name.ends_with(['.', ' ']) {
                return Err(error(tool, "ambiguous Windows path component"));
            }
        }
        relative.push(name);
        resolved.push(name);
        if missing {
            continue;
        }
        match fs::symlink_metadata(&resolved) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(error(
                    tool,
                    "mutation paths must not traverse symbolic links",
                ));
            }
            Ok(_) => resolved = resolve(root, &relative, tool)?,
            Err(err) if err.kind() == io::ErrorKind::NotFound => missing = true,
            Err(err) => return Err(error(tool, err)),
        }
    }
    if resolved == root {
        return Err(error(tool, "cannot modify the workspace root"));
    }
    Ok(resolved)
}

pub(super) fn require_missing(path: &Path, tool: &str) -> Result<(), Error> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(error(tool, "destination already exists")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(error(tool, err)),
    }
}

pub(super) fn write_text(
    path: &Path,
    content: &str,
    existing: Option<Metadata>,
    tool: &str,
) -> Result<(), Error> {
    let mut temporary = temporary_file(path, tool)?;
    temporary
        .write_all(content.as_bytes())
        .map_err(|err| error(tool, err))?;
    if let Some(metadata) = existing {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())
            .map_err(|err| error(tool, err))?;
        temporary
            .persist(path)
            .map_err(|err| error(tool, err.error))?;
    } else {
        temporary
            .persist_noclobber(path)
            .map_err(|err| error(tool, err.error))?;
    }
    Ok(())
}

pub(super) fn temporary_file(path: &Path, tool: &str) -> Result<NamedTempFile, Error> {
    NamedTempFile::new_in(path.parent().expect("mutation path has a parent"))
        .map_err(|err| error(tool, err))
}
