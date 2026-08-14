//! Tauri command wrappers for the post-save preview, kept out of `lib.rs`
//! (already at the harness's 350-line file-size ceiling) following the
//! `editor::commands` precedent.

use super::PreviewPayload;
use crate::capture::CaptureService;

/// Everything the preview needs about saved replay `id`: display name,
/// duration (omitted when the bundle never recorded one), total bytes, and
/// the absolute video path. `id` is resolved against the CURRENT save
/// destination through the library traversal guard, so a request for a
/// bundle outside it fails with `library_invalid_id` rather than reading
/// anything.
#[tauri::command]
pub(crate) fn preview_payload(
    service: tauri::State<'_, CaptureService>,
    id: String,
) -> Result<PreviewPayload, String> {
    super::build(&service.resolved_save_destination(), &id)
}
