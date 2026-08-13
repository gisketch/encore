use super::DiagnosticEntry;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

/// Owns the active file handle plus its tracked size so `append` can decide
/// to rotate without a `stat` on every call. `file` is `None` whenever the
/// path can't be opened (or the writer is `disabled`); every method degrades
/// to a no-op in that case rather than erroring.
pub(super) struct DiagnosticWriter {
    path: PathBuf,
    rotated_path: PathBuf,
    max_bytes: u64,
    file: Option<File>,
    size: u64,
}

impl DiagnosticWriter {
    pub(super) fn open(path: PathBuf, max_bytes: u64) -> Self {
        let rotated_path = rotated_path_for(&path);
        let (file, size) = open_append(&path);
        Self {
            path,
            rotated_path,
            max_bytes,
            file,
            size,
        }
    }

    #[cfg(test)]
    pub(super) fn disabled() -> Self {
        Self {
            path: PathBuf::new(),
            rotated_path: PathBuf::new(),
            max_bytes: 0,
            file: None,
            size: 0,
        }
    }

    pub(super) fn append(&mut self, entry: &DiagnosticEntry) {
        let Ok(mut line) = serde_json::to_string(entry) else {
            return;
        };
        line.push('\n');
        let bytes = line.len() as u64;
        if self.size > 0 && self.size.saturating_add(bytes) > self.max_bytes {
            self.rotate();
        }
        let Some(file) = self.file.as_mut() else {
            return;
        };
        match file.write_all(line.as_bytes()) {
            Ok(()) => self.size = self.size.saturating_add(bytes),
            Err(_) => {
                // Best-effort: drop the broken handle so later appends stop
                // retrying against it; still a silent no-op for the caller.
                self.file = None;
            }
        }
    }

    fn rotate(&mut self) {
        self.file = None;
        if fs::rename(&self.path, &self.rotated_path).is_err() {
            // The cap outranks history: when the predecessor can't be kept,
            // drop the old contents rather than let the active file grow.
            let _ = fs::remove_file(&self.path);
        }
        let (file, size) = open_append(&self.path);
        self.file = file;
        self.size = size;
    }
}

pub(super) fn rotated_path_for(path: &Path) -> PathBuf {
    let mut rotated = path.as_os_str().to_owned();
    rotated.push(".1");
    PathBuf::from(rotated)
}

fn open_append(path: &Path) -> (Option<File>, u64) {
    if path.as_os_str().is_empty() {
        return (None, 0);
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && fs::create_dir_all(parent).is_err() {
            return (None, 0);
        }
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => {
            let size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
            (Some(file), size)
        }
        Err(_) => (None, 0),
    }
}
