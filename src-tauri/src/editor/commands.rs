//! Tauri command wrappers for the Editor window, kept out of `lib.rs`
//! (already at the harness's 350-line file-size ceiling) rather than
//! grown further — a pure file-boundary split, no behavior change from
//! defining them inline there.

use super::{EditorContext, EditorHeader, EditorKeyframes};
use crate::capture::CaptureService;

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
