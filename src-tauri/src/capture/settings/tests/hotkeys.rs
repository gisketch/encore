use super::*;

#[test]
fn hotkeys_round_trip_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument::new(
        10,
        PersistedTarget::Display,
        "system".into(),
        None,
        "nothing".into(),
        Hotkeys {
            save_replay: "Cmd+Alt+T".into(),
            pause_capture: "Cmd+Shift+P".into(),
            open_library: "Ctrl+Alt+L".into(),
        },
        true,
    );

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
}

#[test]
fn missing_hotkeys_field_defaults_to_the_stock_chords() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"},"appearance":"system"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    assert_eq!(store.load().hotkeys, Hotkeys::default());
}

#[test]
fn invalid_hotkey_accelerator_falls_back_to_its_default_without_losing_the_others() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"},"appearance":"system",
            "hotkeys":{"save_replay":"not a chord","pause_capture":"Cmd+Alt+P","open_library":"Cmd+Alt+L"}}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    let loaded = store.load().hotkeys;
    assert_eq!(loaded.save_replay, Hotkeys::default().save_replay);
    assert_eq!(loaded.pause_capture, "Cmd+Alt+P");
    assert_eq!(loaded.open_library, "Cmd+Alt+L");
}

#[test]
fn valid_hotkey_accelerator_accepts_parseable_chords_and_rejects_garbage() {
    assert!(valid_hotkey_accelerator("Cmd+Alt+R"));
    assert!(valid_hotkey_accelerator("Ctrl+Shift+L"));
    assert!(!valid_hotkey_accelerator("not a chord"));
    assert!(!valid_hotkey_accelerator(""));
}
