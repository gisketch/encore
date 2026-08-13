//! Tauri command wrappers for the Editor window, kept out of `lib.rs`
//! (already at the harness's 350-line file-size ceiling) rather than
//! grown further — a pure file-boundary split, no behavior change from
//! defining them inline there.

use super::{clipboard, EditorContext, EditorHeader, EditorKeyframes};
use crate::capture::CaptureService;
use std::path::Path;

/// Validates `id`, records it as the Editor window's current replay, grants
/// the asset protocol read access it needs, then shows/focuses the window
/// (`desktop::open_editor_window`, mirroring `open_library_window`).
#[tauri::command]
pub(crate) fn open_editor_window(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    context: tauri::State<'_, EditorContext>,
    id: String,
) -> Result<(), String> {
    super::open(
        &app,
        &service.resolved_save_destination(),
        context.inner(),
        id,
    )?;
    crate::desktop::open_editor_window(&app)
}

/// The Editor window's own bootstrap read: which replay id
/// `open_editor_window` most recently recorded, if any.
#[tauri::command]
pub(crate) fn editor_context(context: tauri::State<'_, EditorContext>) -> Option<String> {
    super::current(context.inner())
}

#[tauri::command]
pub(crate) fn editor_header(
    service: tauri::State<'_, CaptureService>,
    id: String,
) -> Result<EditorHeader, String> {
    super::header(&service.resolved_save_destination(), &id)
}

#[tauri::command]
pub(crate) fn editor_keyframes(
    service: tauri::State<'_, CaptureService>,
    id: String,
) -> Result<EditorKeyframes, String> {
    super::keyframes(&service.resolved_save_destination(), &id)
}

/// Places a file-URL reference to `path` on the macOS pasteboard, after
/// verifying `path` really resolves inside the current save destination —
/// the frontend already exported the file and is handing back its own
/// path, but this command never trusts frontend input by construction.
#[tauri::command]
pub(crate) fn copy_export_to_clipboard(
    service: tauri::State<'_, CaptureService>,
    path: String,
) -> Result<(), String> {
    let destination = service.resolved_save_destination();
    let guarded = clipboard::guard_within_destination(&destination, Path::new(&path))?;
    clipboard::copy_to_clipboard(&guarded)
}
