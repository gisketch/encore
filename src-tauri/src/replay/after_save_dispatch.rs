use super::{
    after_save_choice::{action_for, AfterSaveAction},
    shortcut::reveal_and_emit,
    ReplayService, SavedReplaySnapshot,
};
use crate::capture::CaptureService;
use tauri::{AppHandle, Manager};

/// Applies the persisted after-save choice for a just-saved replay. Split
/// out of `after_save::honor_after_save` so that tracked file's own
/// branching count stays at its committed baseline while this ticket adds a
/// third choice (`open_editor`). The choice-string-to-action mapping itself
/// lives in `after_save_choice`, where it is testable without an
/// `AppHandle`.
pub(super) fn apply(
    app: &AppHandle,
    service: &ReplayService,
    capture: &CaptureService,
    saved: &SavedReplaySnapshot,
) {
    match action_for(&capture.after_save()) {
        AfterSaveAction::Reveal => {
            let _ = reveal_and_emit(app, service, &saved.id);
        }
        // `saved.display_name`, not `saved.id`: see `open_in_editor`.
        AfterSaveAction::OpenEditor => open_in_editor(app, capture, &saved.display_name),
        // PP-01 routes the choice only; PP-02 attaches the preview window
        // here, reading `preview::payload` for the just-saved `saved.id`.
        AfterSaveAction::Preview => {}
        AfterSaveAction::Nothing => {}
    }
}

/// Opens the just-saved replay in the Editor window: validates `bundle_id`
/// and grants asset-protocol access via the same `editor::open` seam the
/// Library's "Open in editor" button uses, then shows/focuses the window.
/// Any failure (missing window state, traversal rejection) is silent here —
/// the save itself already succeeded and stays unaffected.
///
/// `bundle_id` must be the bundle's folder name, which is
/// `SavedReplaySnapshot::display_name` — NOT its `id`. A replay carries two
/// identifiers: `id` is a session-scoped counter (`replay-1`) that only the
/// in-memory replay state understands (it is what `reveal_saved` looks up),
/// while the Library, Editor, and preview all address a replay by the
/// folder the packager published it into (`destination/{display_name}`).
/// Passing `id` here silently resolves to a non-existent folder.
fn open_in_editor(app: &AppHandle, capture: &CaptureService, bundle_id: &str) {
    let Some(context) = app.try_state::<crate::editor::EditorContext>() else {
        return;
    };
    let destination = capture.resolved_save_destination();
    if crate::editor::open(app, &destination, context.inner(), bundle_id.to_string()).is_ok() {
        let _ = crate::desktop::open_editor_window(app);
    }
}
