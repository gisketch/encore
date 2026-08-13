mod delete;
mod group;
mod guard;
mod scan;
#[cfg(test)]
mod tests;
mod thumbnail;

#[allow(unused_imports)]
pub use group::{LibraryEntry, LibraryGroup};
/// Re-exported for the `editor` module, which resolves the same
/// frontend-supplied bundle ids through this one traversal guard rather
/// than duplicating it.
#[allow(unused_imports)]
pub(crate) use guard::{resolve_bundle_dir, resolve_replay_file};

use base64::{engine::general_purpose::STANDARD, Engine};
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

/// Moves bundle `id`'s whole folder (video + metadata) to the macOS Trash,
/// after `id` clears the same traversal guard `open_replay_file` uses.
/// `cache_dir` is best-effort: when present, the bundle's thumbnail cache
/// entry is forgotten too, but a missing cache dir never blocks the
/// delete itself.
pub fn delete_replay(destination: &Path, cache_dir: Option<&Path>, id: &str) -> Result<(), String> {
    delete::delete_bundle(destination, cache_dir, id, &delete::SystemTrashMover)
}

/// Returns bundle `id`'s cached thumbnail as base64 JPEG (generating it on
/// a cache miss), for the `library_thumbnail` command to hand straight to
/// an `<img src="data:image/jpeg;base64,…">`. Base64-over-the-wire, rather
/// than widening the Tauri asset-protocol scope to the cache directory, is
/// the smaller change for ~320px JPEGs.
pub fn thumbnail_base64(destination: &Path, cache_dir: &Path, id: &str) -> Result<String, String> {
    let ffmpeg = crate::packager::current_sidecar_path("ffmpeg")
        .map_err(|_| "library_thumbnail_unavailable".to_string())?;
    let extractor = thumbnail::ProcessThumbnailExtractor::new(ffmpeg);
    thumbnail::thumbnail_bytes(destination, cache_dir, id, &extractor)
        .map(|bytes| STANDARD.encode(bytes))
}
