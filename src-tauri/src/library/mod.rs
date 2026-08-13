mod group;
mod guard;
mod scan;
#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use group::{LibraryEntry, LibraryGroup};

use chrono::Local;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryIndex {
    pub groups: Vec<LibraryGroup>,
    pub total_count: u32,
    pub total_bytes: u64,
}

/// Scans `destination` (the CURRENT resolved save destination — callers
/// must read it fresh each time, never cache it) and returns the day-
/// grouped index the Library window renders directly. The folder is the
/// only source of truth: nothing here is cached between calls, so
/// externally added or removed bundles show up on the next scan.
pub fn index(destination: &Path) -> LibraryIndex {
    let entries = scan::scan(destination);
    let total_count = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    let total_bytes = entries.iter().map(|entry| entry.total_bytes).sum();
    let today = Local::now().date_naive();
    let groups = group::group_by_day(entries, today);
    LibraryIndex {
        groups,
        total_count,
        total_bytes,
    }
}

/// Opens a bundle's replay file in the system default player. `id` is
/// untrusted frontend input, so every path stays behind
/// `guard::resolve_replay_file`'s traversal check.
pub fn open_replay_file(destination: &Path, id: &str) -> Result<(), String> {
    let file = guard::resolve_replay_file(destination, id)?;
    if !file.is_file() {
        return Err("library_replay_missing".to_string());
    }
    std::process::Command::new("open")
        .arg(&file)
        .spawn()
        .map(|_| ())
        .map_err(|_| "library_open_failed".to_string())
}
