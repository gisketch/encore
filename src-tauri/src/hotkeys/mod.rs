mod dispatch;
#[cfg(test)]
mod tests;

use crate::capture::{CaptureService, HotkeyId, Hotkeys, SettingsSnapshot};
use crate::replay::{self, ReplayService, ShortcutRegistrationSnapshot, ShortcutRegistrationState};
use std::str::FromStr;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// The registration seam: production wraps a live `AppHandle`'s global
/// shortcut plugin, tests inject a fake that can be told to fail on
/// command. Keeping this as a trait (rather than calling
/// `app.global_shortcut()` directly throughout the module) is what makes
/// `register_startup`/`swap_registration` testable without a live Tauri app.
pub(crate) trait HotkeyRegistrar {
    fn register(&self, id: HotkeyId, accelerator: &str) -> Result<(), String>;
    fn unregister(&self, accelerator: &str) -> Result<(), String>;
}

pub(crate) struct AppHandleRegistrar<'a>(pub(crate) &'a AppHandle);

impl HotkeyRegistrar for AppHandleRegistrar<'_> {
    fn register(&self, id: HotkeyId, accelerator: &str) -> Result<(), String> {
        self.0
            .global_shortcut()
            .on_shortcut(accelerator, move |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    dispatch::dispatch(app, id);
                }
            })
            .map_err(|_| "hotkey_registration_failed".to_string())
    }

    fn unregister(&self, accelerator: &str) -> Result<(), String> {
        self.0
            .global_shortcut()
            .unregister(accelerator)
            .map_err(|_| "hotkey_unregister_failed".to_string())
    }
}

/// Registers all three hotkeys from a set of accelerators, independently:
/// one failing to register never blocks the others. Returns each hotkey's
/// outcome so the caller can report or mirror it without this function
/// needing to know about `CaptureService`/`ReplayService` itself — the pure
/// core of `register_all`, directly testable with a fake registrar.
pub(crate) fn register_startup(
    registrar: &dyn HotkeyRegistrar,
    hotkeys: &Hotkeys,
) -> [(HotkeyId, Result<(), String>); 3] {
    [
        (
            HotkeyId::SaveReplay,
            registrar.register(HotkeyId::SaveReplay, &hotkeys.save_replay),
        ),
        (
            HotkeyId::PauseCapture,
            registrar.register(HotkeyId::PauseCapture, &hotkeys.pause_capture),
        ),
        (
            HotkeyId::OpenLibrary,
            registrar.register(HotkeyId::OpenLibrary, &hotkeys.open_library),
        ),
    ]
}

/// Unregisters `previous`, tries `next`; on failure, re-registers `previous`
/// so the OS binding is never left empty. Pure over the registrar trait —
/// the other directly-testable core, shared by `update_hotkey`.
pub(crate) fn swap_registration(
    registrar: &dyn HotkeyRegistrar,
    id: HotkeyId,
    previous: &str,
    next: &str,
) -> Result<(), String> {
    let _ = registrar.unregister(previous);
    if let Err(code) = registrar.register(id, next) {
        let _ = registrar.register(id, previous);
        return Err(code);
    }
    Ok(())
}

pub(crate) fn registration_snapshot(result: &Result<(), String>) -> ShortcutRegistrationSnapshot {
    match result {
        Ok(()) => ShortcutRegistrationSnapshot {
            state: ShortcutRegistrationState::Registered,
            error_code: None,
        },
        Err(code) => ShortcutRegistrationSnapshot {
            state: ShortcutRegistrationState::Unavailable,
            error_code: Some(code.clone()),
        },
    }
}

/// Registers all three hotkeys from their persisted accelerators. Called
/// once at startup; `save_replay`'s outcome is mirrored onto the replay
/// snapshot's existing shortcut surface, which the bar already renders.
pub(crate) fn register_all(
    registrar: &dyn HotkeyRegistrar,
    capture: &CaptureService,
    replay: &ReplayService,
) {
    for (id, outcome) in register_startup(registrar, &capture.hotkeys()) {
        if id == HotkeyId::SaveReplay {
            replay.set_shortcut_registration(registration_snapshot(&outcome));
        }
    }
}

/// Rebinds one hotkey via `swap_registration`, then persists only on
/// success, per PG-07's "registration is attempted only on confirm"
/// contract — a chord being recorded in the Settings window never reaches
/// this function until the user stops typing and it resolves to a
/// candidate.
pub(crate) fn update_hotkey(
    registrar: &dyn HotkeyRegistrar,
    app: &AppHandle,
    capture: &CaptureService,
    replay: &ReplayService,
    id: HotkeyId,
    accelerator: String,
) -> Result<SettingsSnapshot, String> {
    if Shortcut::from_str(&accelerator).is_err() {
        return Err("hotkey_invalid".into());
    }
    let previous = capture.hotkey(id);
    swap_registration(registrar, id, &previous, &accelerator)?;
    let snapshot = capture.set_hotkey(id, accelerator)?;
    if id == HotkeyId::SaveReplay {
        replay.set_shortcut_registration(registration_snapshot(&Ok(())));
        replay::emit_snapshot(app, replay);
    }
    Ok(snapshot)
}
