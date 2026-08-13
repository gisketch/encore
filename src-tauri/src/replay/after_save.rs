use super::{shortcut::reveal_and_emit, ReplayService, ReplaySnapshot, ReplayState};
use crate::capture::CaptureService;
use tauri::{AppHandle, Manager};

/// Honors the persisted after-save choice at the exact point a save reaches
/// the `saved` state: `reveal` triggers the same reveal machinery the bar's
/// manual button uses, `nothing` preserves today's silent behavior. A
/// missing `CaptureService` (should not happen once `lib.rs` wires it) or
/// any other state just does nothing, leaving the save itself unaffected.
pub(super) fn honor_after_save(
    app: &AppHandle,
    service: &ReplayService,
    snapshot: &ReplaySnapshot,
) {
    let Some(saved) = (snapshot.state == ReplayState::Saved)
        .then_some(snapshot.saved.as_ref())
        .flatten()
    else {
        return;
    };
    let Some(capture) = app.try_state::<CaptureService>() else {
        return;
    };
    if capture.after_save() == "reveal" {
        let _ = reveal_and_emit(app, service, &saved.id);
    }
}
