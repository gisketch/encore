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
    // Deliberately outside the match: the confirmation sound is its own
    // setting, so `after_save = nothing` still gets it. Reached only for a
    // save that succeeded, so failures stay silent.
    crate::sound::play_saved_chime(app, capture.save_sound());
    match action_for(&capture.after_save()) {
        AfterSaveAction::Reveal => {
            let _ = reveal_and_emit(app, service, &saved.id);
        }
        // `saved.display_name`, not `saved.id`: see `open_in_editor`.
        AfterSaveAction::OpenEditor => open_in_editor(app, capture, &saved.display_name),
        // `saved.display_name` again, for the reason `open_in_editor`
        // spells out: the preview addresses the bundle on disk.
        AfterSaveAction::Preview => show_preview(app, capture, &saved.display_name),
        AfterSaveAction::Nothing => {}
    }
}

/// Raises the corner preview for the just-saved replay. Like
/// `open_in_editor`, every failure is swallowed: the save has already
/// succeeded and its recorded state must not depend on whether a preview
/// could be built or shown.
fn show_preview(app: &AppHandle, capture: &CaptureService, bundle_id: &str) {
    let _ = crate::preview::show(app, &capture.resolved_save_destination(), bundle_id);
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
