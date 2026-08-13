pub(crate) mod commands;
pub(crate) mod export;
mod header;
mod keyframes;

use std::{path::Path, sync::Mutex};
use tauri::Manager;

#[allow(unused_imports)]
pub(crate) use header::EditorHeader;
#[allow(unused_imports)]
pub(crate) use keyframes::EditorKeyframes;

/// The editor window's current replay id. A single hidden window is
/// reused across opens (mirroring settings/library), so "which replay is
/// open" lives here rather than in the window itself — the editor reads it
/// on mount via the `editor_context` command.
pub(crate) type EditorContext = Mutex<Option<String>>;

/// Validates `id` against `destination` (the same traversal guard
/// `open_replay_file` uses), records it as the editor's current replay, and
/// grants the asset protocol read access to `destination` so the editor's
/// `<video>` element can load the file directly via `convertFileSrc`. The
/// asset protocol scope is process-wide (Tauri has no per-window scope), so
/// this only ever grants the resolved save destination tree — never
/// anything wider — and re-granting the same directory on every open is a
/// harmless no-op.
pub(crate) fn open(
    app: &tauri::AppHandle,
    destination: &Path,
    context: &EditorContext,
    id: String,
) -> Result<(), String> {
    let replay_file = crate::library::resolve_replay_file(destination, &id)?;
    if !replay_file.is_file() {
        return Err("editor_replay_missing".to_string());
    }
    app.asset_protocol_scope()
        .allow_directory(destination, true)
        .map_err(|_| "editor_scope_failed".to_string())?;
    *context
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(id);
    Ok(())
}

/// The editor window's current replay id, if one has been opened this
/// session.
pub(crate) fn current(context: &EditorContext) -> Option<String> {
    context
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

pub(crate) fn header(destination: &Path, id: &str) -> Result<EditorHeader, String> {
    header::build(destination, id)
}

/// Runs the bundled ffprobe sidecar (the same resolution `library::
/// thumbnail` uses for ffmpeg) against `id`'s replay file and returns its
/// keyframe timestamps and duration.
pub(crate) fn keyframes(destination: &Path, id: &str) -> Result<EditorKeyframes, String> {
    let replay_file = crate::library::resolve_replay_file(destination, id)?;
    let ffprobe = crate::packager::current_sidecar_path("ffprobe")
        .map_err(|_| "editor_ffprobe_unavailable".to_string())?;
    keyframes::probe(&replay_file, &keyframes::ProcessKeyframeProbe::new(ffprobe))
}
