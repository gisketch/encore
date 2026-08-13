use super::{shortcut::reveal_and_emit, ReplayService, SavedReplaySnapshot};
use crate::capture::CaptureService;
use tauri::{AppHandle, Manager};

/// Applies the persisted after-save choice for a just-saved replay. Split
/// out of `after_save::honor_after_save` so that tracked file's own
/// branching count stays at its committed baseline while this ticket adds a
/// third choice (`open_editor`).
pub(super) fn apply(
    app: &AppHandle,
    service: &ReplayService,
    capture: &CaptureService,
    saved: &SavedReplaySnapshot,
) {
    match capture.after_save().as_str() {
        "reveal" => {
            let _ = reveal_and_emit(app, service, &saved.id);
        }
        "open_editor" => open_in_editor(app, capture, &saved.id),
        _ => {}
    }
}

/// Opens the just-saved replay in the Editor window: validates `id` and
/// grants asset-protocol access via the same `editor::open` seam the
/// Library's "Open in editor" button uses, then shows/focuses the window.
/// Any failure (missing window state, traversal rejection) is silent here —
/// the save itself already succeeded and stays unaffected.
fn open_in_editor(app: &AppHandle, capture: &CaptureService, id: &str) {
    let Some(context) = app.try_state::<crate::editor::EditorContext>() else {
        return;
    };
    let destination = capture.resolved_save_destination();
    if crate::editor::open(app, &destination, context.inner(), id.to_string()).is_ok() {
        let _ = crate::desktop::open_editor_window(app);
    }
}
