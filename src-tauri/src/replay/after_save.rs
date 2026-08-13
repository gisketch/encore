use super::{after_save_dispatch, ReplayService, ReplaySnapshot, ReplayState};
use crate::capture::CaptureService;
use tauri::{AppHandle, Manager};

/// Honors the persisted after-save choice at the exact point a save reaches
/// the `saved` state. The actual per-choice behavior lives in
/// `after_save_dispatch::apply` so this file's own branching stays at its
/// committed baseline as choices are added. A missing `CaptureService`
/// (should not happen once `lib.rs` wires it) or any other replay state
/// just does nothing, leaving the save itself unaffected.
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
    after_save_dispatch::apply(app, service, &capture, saved);
}
