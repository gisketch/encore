use super::HotkeyId;
use crate::capture::{CaptureService, CaptureState};
use crate::replay::{self, ReplayService};
use std::thread;
use tauri::{AppHandle, Manager};

/// Routes a fired global hotkey to the same action its bar counterpart
/// drives, off the OS callback thread — mirroring `replay::shortcut`'s own
/// dispatch pattern — so a slow action never blocks the next key event.
pub(super) fn dispatch(app: &AppHandle, id: HotkeyId) {
    let app = app.clone();
    thread::spawn(move || match id {
        HotkeyId::SaveReplay => dispatch_save_replay(&app),
        HotkeyId::PauseCapture => dispatch_pause_capture(&app),
        HotkeyId::OpenLibrary => dispatch_open_library(&app),
    });
}

fn dispatch_save_replay(app: &AppHandle) {
    let Some(replay) = app.try_state::<ReplayService>() else {
        return;
    };
    let _ = replay::trigger_and_emit(app, replay.inner());
    crate::desktop::show_window_without_focus(app);
}

/// Toggles pause/resume based on the live capture state, same as the bar's
/// advanced-row button and the tray menu.
fn dispatch_pause_capture(app: &AppHandle) {
    let Some(capture) = app.try_state::<CaptureService>() else {
        return;
    };
    let _ = if capture.snapshot().capture == CaptureState::Paused {
        capture.resume()
    } else {
        capture.pause()
    };
}

fn dispatch_open_library(app: &AppHandle) {
    let Some(capture) = app.try_state::<CaptureService>() else {
        return;
    };
    let _ = capture.open_library();
}
