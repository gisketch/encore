use crate::packager::PackageFileSystem;
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static PARTIAL_NONCE: AtomicU64 = AtomicU64::new(1);
const MAX_NAME_SUFFIX: u32 = 10_000;

/// Publishes a trimmed export as a sibling bundle in `destination`, named
/// `"{source_name} (trimmed)"` (deduped with " 2", " 3", ... on
/// collision). Deliberately the same lock-then-partial-dir-then-rename
/// dance as `packager::package::ReplayPackager::package` — that struct is
/// private to the packager module, so this is a small, independent copy
/// of the pattern (same convention `editor::header::title` already uses
/// for a shared-but-private helper) rather than a cross-module export,
/// built on the SAME `PackageFileSystem` trait so both publishers go
/// through identical atomic-write primitives. `write_bundle` fills the
/// hidden partial directory; only once it returns `Ok` does the directory
/// get renamed into place, so a crash or failure never leaves a
/// half-written bundle visible to the Library scan.
pub(crate) fn publish_trimmed<F>(
    files: &dyn PackageFileSystem,
    destination: &Path,
    source_name: &str,
    write_bundle: F,
) -> Result<PathBuf, String>
where
    F: Fn(&Path) -> Result<(), String>,
{
    files
        .create_dir_all(destination)
        .map_err(|_| "export_destination_unavailable".to_string())?;

    for suffix in 0..=MAX_NAME_SUFFIX {
        let name = trimmed_display_name(source_name, suffix);
        let completed = destination.join(&name);
        if exists(files, &completed)? {
            continue;
        }

        let lock = destination.join(format!(".{name}.lock-{}", std::process::id()));
        match files.create_lock(&lock) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("export_destination_unavailable".into()),
        }
        let partial = destination.join(format!(
            ".{name}.partial-{}-{}",
            std::process::id(),
            PARTIAL_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut workspace = match PartialExportWorkspace::new(files, partial, lock.clone()) {
            Ok(workspace) => workspace,
            Err(code) => {
                let _ = files.remove_file(&lock);
                return Err(code);
            }
        };

        if exists(files, &completed)? {
            continue;
        }
        write_bundle(workspace.directory())?;
        if exists(files, &completed)? {
            continue;
        }
        match files.rename(workspace.directory(), &completed) {
            Ok(()) => {}
            Err(_) if files.try_exists(&completed).unwrap_or(false) => continue,
            Err(_) => return Err("export_destination_unavailable".into()),
        }
        workspace.mark_published();
        files
            .sync_dir(destination)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        return Ok(completed);
    }
    Err("export_destination_unavailable".into())
}

fn exists(files: &dyn PackageFileSystem, path: &Path) -> Result<bool, String> {
    files
        .try_exists(path)
        .map_err(|_| "export_destination_unavailable".to_string())
}

fn trimmed_display_name(source_name: &str, suffix: u32) -> String {
    let suffix_text = if suffix == 0 {
        String::new()
    } else {
        format!(" {suffix}")
    };
    format!("{source_name} (trimmed){suffix_text}")
}

struct PartialExportWorkspace<'a> {
    files: &'a dyn PackageFileSystem,
    directory: PathBuf,
    lock: PathBuf,
    published: bool,
}

impl<'a> PartialExportWorkspace<'a> {
    fn new(
        files: &'a dyn PackageFileSystem,
        directory: PathBuf,
        lock: PathBuf,
    ) -> Result<Self, String> {
        files
            .create_dir(&directory)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        Ok(Self {
            files,
            directory,
            lock,
            published: false,
        })
    }

    fn directory(&self) -> &Path {
        &self.directory
    }

    fn mark_published(&mut self) {
        self.published = true;
    }
}

impl Drop for PartialExportWorkspace<'_> {
    fn drop(&mut self) {
        if !self.published {
            let _ = self.files.remove_dir_all(&self.directory);
        }
        let _ = self.files.remove_file(&self.lock);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_zero_has_no_trailing_number() {
        assert_eq!(
            trimmed_display_name("Encore Replay X", 0),
            "Encore Replay X (trimmed)"
        );
    }

    #[test]
    fn later_suffixes_dedupe_like_the_packager() {
        assert_eq!(
            trimmed_display_name("Encore Replay X", 2),
            "Encore Replay X (trimmed) 2"
        );
    }
}
