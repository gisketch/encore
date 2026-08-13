use super::{ReplayDispatch, ReplayService, ReplaySnapshot};
use std::thread;
use tauri::{AppHandle, Emitter};

const REPLAY_STATE_CHANGED_EVENT: &str = "replay-state-changed";

pub(crate) fn trigger_and_emit(
    app: &AppHandle,
    service: &ReplayService,
) -> Result<ReplaySnapshot, String> {
    let dispatch = service.trigger_for_export();
    emit_dispatch(app, service, dispatch)
}

pub(crate) fn retry_and_emit(
    app: &AppHandle,
    service: &ReplayService,
) -> Result<ReplaySnapshot, String> {
    let dispatch = service.retry_for_export();
    emit_dispatch(app, service, dispatch)
}

pub(crate) fn reveal_and_emit(
    app: &AppHandle,
    service: &ReplayService,
    replay_id: &str,
) -> Result<ReplaySnapshot, String> {
    let result = service.reveal_saved(replay_id);
    let snapshot = result.clone().unwrap_or_else(|_| service.snapshot());
    emit_replay_state(app, snapshot);
    result
}

fn emit_dispatch(
    app: &AppHandle,
    service: &ReplayService,
    dispatch: Result<ReplayDispatch, String>,
) -> Result<ReplaySnapshot, String> {
    match dispatch {
        Ok(dispatch) => {
            let snapshot = dispatch.snapshot;
            emit_replay_state(app, snapshot.clone());
            if let Some(replay_id) = dispatch.replay_id {
                dispatch_export(app.clone(), service.clone(), replay_id);
            }
            Ok(snapshot)
        }
        Err(error) => {
            emit_replay_state(app, service.snapshot());
            Err(error)
        }
    }
}

fn dispatch_export(app: AppHandle, service: ReplayService, replay_id: String) {
    thread::spawn(move || {
        let snapshot = service.run_export(&replay_id);
        emit_replay_state(&app, snapshot.clone());
        super::after_save::honor_after_save(&app, &service, &snapshot);
    });
}

fn emit_replay_state(app: &AppHandle, snapshot: ReplaySnapshot) {
    let _ = app.emit(REPLAY_STATE_CHANGED_EVENT, snapshot);
}

/// Broadcasts the current replay snapshot, for callers outside this module
/// that just changed something the snapshot reflects (e.g. the hotkeys
/// registrar updating `save_replay`'s registration state).
pub(crate) fn emit_snapshot(app: &AppHandle, service: &ReplayService) {
    emit_replay_state(app, service.snapshot());
}
