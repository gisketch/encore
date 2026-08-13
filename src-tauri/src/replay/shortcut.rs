use super::{
    ReplayDispatch, ReplayService, ReplaySnapshot, ShortcutRegistrationSnapshot,
    ShortcutRegistrationState,
};
use crate::desktop;
use std::thread;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const REPLAY_SHORTCUT: &str = "CommandOrControl+Alt+R";
const REPLAY_STATE_CHANGED_EVENT: &str = "replay-state-changed";

pub(crate) fn register_global_shortcut(app: &AppHandle, service: &ReplayService) {
    let registration =
        app.global_shortcut()
            .on_shortcut(REPLAY_SHORTCUT, |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    dispatch_global_replay(app.clone());
                }
            });
    service.set_shortcut_registration(shortcut_registration(registration.is_ok()));
    emit_replay_state(app, service.snapshot());
}

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

fn dispatch_global_replay(app: AppHandle) {
    thread::spawn(move || {
        let service = app.state::<ReplayService>().inner().clone();
        let _ = trigger_and_emit(&app, &service);
        desktop::show_window_without_focus(&app);
    });
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
        emit_replay_state(&app, snapshot);
    });
}

fn shortcut_registration(registered: bool) -> ShortcutRegistrationSnapshot {
    if registered {
        ShortcutRegistrationSnapshot {
            state: ShortcutRegistrationState::Registered,
            error_code: None,
        }
    } else {
        ShortcutRegistrationSnapshot {
            state: ShortcutRegistrationState::Unavailable,
            error_code: Some("shortcut_registration_failed".into()),
        }
    }
}

fn emit_replay_state(app: &AppHandle, snapshot: ReplaySnapshot) {
    let _ = app.emit(REPLAY_STATE_CHANGED_EVENT, snapshot);
}
