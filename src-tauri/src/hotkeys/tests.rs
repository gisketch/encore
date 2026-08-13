use super::*;
use crate::capture::Hotkeys;
use std::sync::Mutex;

/// Records every register/unregister call and can be told to fail one
/// specific hotkey's next `register` call, so tests can exercise both the
/// happy path and the rollback path without a live Tauri app.
#[derive(Default)]
struct FakeRegistrar {
    fail_next_register_for: Mutex<Option<HotkeyId>>,
    registered: Mutex<Vec<(HotkeyId, String)>>,
    unregistered: Mutex<Vec<String>>,
}

impl FakeRegistrar {
    fn failing(id: HotkeyId) -> Self {
        Self {
            fail_next_register_for: Mutex::new(Some(id)),
            ..Self::default()
        }
    }
}

impl HotkeyRegistrar for FakeRegistrar {
    fn register(&self, id: HotkeyId, accelerator: &str) -> Result<(), String> {
        self.registered
            .lock()
            .unwrap()
            .push((id, accelerator.to_string()));
        let mut fail_next = self.fail_next_register_for.lock().unwrap();
        if *fail_next == Some(id) {
            *fail_next = None;
            return Err("hotkey_registration_failed".into());
        }
        Ok(())
    }

    fn unregister(&self, accelerator: &str) -> Result<(), String> {
        self.unregistered
            .lock()
            .unwrap()
            .push(accelerator.to_string());
        Ok(())
    }
}

fn hotkeys() -> Hotkeys {
    Hotkeys {
        save_replay: "Cmd+Alt+R".into(),
        pause_capture: "Cmd+Alt+P".into(),
        open_library: "Cmd+Alt+L".into(),
    }
}

#[test]
fn register_startup_attempts_all_three_hotkeys() {
    let registrar = FakeRegistrar::default();

    let outcomes = register_startup(&registrar, &hotkeys());

    assert!(outcomes.iter().all(|(_, outcome)| outcome.is_ok()));
    let registered = registrar.registered.lock().unwrap();
    assert_eq!(registered.len(), 3);
    assert!(registered.contains(&(HotkeyId::SaveReplay, "Cmd+Alt+R".to_string())));
    assert!(registered.contains(&(HotkeyId::PauseCapture, "Cmd+Alt+P".to_string())));
    assert!(registered.contains(&(HotkeyId::OpenLibrary, "Cmd+Alt+L".to_string())));
}

#[test]
fn register_startup_reports_one_failure_without_blocking_the_others() {
    let registrar = FakeRegistrar::failing(HotkeyId::PauseCapture);

    let outcomes = register_startup(&registrar, &hotkeys());

    let by_id = |id: HotkeyId| {
        outcomes
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .unwrap()
    };
    assert!(by_id(HotkeyId::SaveReplay).1.is_ok());
    assert!(by_id(HotkeyId::PauseCapture).1.is_err());
    assert!(by_id(HotkeyId::OpenLibrary).1.is_ok());
}

#[test]
fn swap_registration_unregisters_previous_and_registers_next_on_success() {
    let registrar = FakeRegistrar::default();

    let result = swap_registration(&registrar, HotkeyId::SaveReplay, "Cmd+Alt+R", "Cmd+Alt+T");

    assert!(result.is_ok());
    assert_eq!(
        *registrar.unregistered.lock().unwrap(),
        vec!["Cmd+Alt+R".to_string()]
    );
    assert_eq!(
        *registrar.registered.lock().unwrap(),
        vec![(HotkeyId::SaveReplay, "Cmd+Alt+T".to_string())]
    );
}

#[test]
fn swap_registration_rolls_back_to_the_previous_accelerator_on_failure() {
    let registrar = FakeRegistrar::failing(HotkeyId::SaveReplay);

    let result = swap_registration(&registrar, HotkeyId::SaveReplay, "Cmd+Alt+R", "Cmd+Alt+T");

    assert_eq!(result, Err("hotkey_registration_failed".to_string()));
    let registered = registrar.registered.lock().unwrap();
    // First attempt (the failing "next" chord), then the rollback attempt
    // re-registering the previous chord.
    assert_eq!(
        *registered,
        vec![
            (HotkeyId::SaveReplay, "Cmd+Alt+T".to_string()),
            (HotkeyId::SaveReplay, "Cmd+Alt+R".to_string()),
        ]
    );
}

#[test]
fn registration_snapshot_reflects_success_and_failure() {
    let ok = registration_snapshot(&Ok(()));
    assert_eq!(ok.state, ShortcutRegistrationState::Registered);
    assert_eq!(ok.error_code, None);

    let err = registration_snapshot(&Err("hotkey_registration_failed".to_string()));
    assert_eq!(err.state, ShortcutRegistrationState::Unavailable);
    assert_eq!(
        err.error_code,
        Some("hotkey_registration_failed".to_string())
    );
}
