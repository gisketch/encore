//! Tauri command wrappers for the Library window, kept out of `lib.rs`
//! (already at the harness's 350-line file-size ceiling) rather than grown
//! further — a pure file-boundary move mirroring `editor::commands`, with
//! no behavior change from defining them inline there.

use super::LibraryIndex;
use crate::capture::CaptureService;
use tauri::Manager;

#[tauri::command]
pub(crate) fn open_library_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::desktop::open_library_window(&app)
}

#[tauri::command]
pub(crate) fn library_index(service: tauri::State<'_, CaptureService>) -> LibraryIndex {
    super::index(&service.resolved_save_destination())
}

#[tauri::command]
pub(crate) fn open_replay_file(
    service: tauri::State<'_, CaptureService>,
    id: String,
) -> Result<(), String> {
    super::open_replay_file(&service.resolved_save_destination(), &id)
}

/// Moves bundle `id`'s folder to the macOS Trash. The app cache dir is
/// looked up the same way `library_thumbnail` does; when unavailable, the
/// delete still proceeds — only the (best-effort) cache cleanup is skipped.
#[tauri::command]
pub(crate) fn delete_replay(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    id: String,
) -> Result<(), String> {
    let cache_dir = app.path().app_cache_dir().ok();
    super::delete_replay(
        &service.resolved_save_destination(),
        cache_dir.as_deref(),
        &id,
    )
}

/// Returns bundle `id`'s cached thumbnail as base64 JPEG, generating it on
/// a cache miss. Cards call this lazily after the list has already
/// rendered, so a miss (including "no cache dir available") never blocks
/// the index — the frontend falls back to its styled placeholder.
#[tauri::command]
pub(crate) fn library_thumbnail(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    id: String,
) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|_| "library_thumbnail_unavailable".to_string())?;
    super::thumbnail_base64(&service.resolved_save_destination(), &cache_dir, &id)
}
