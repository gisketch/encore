use super::*;

#[test]
fn missing_menu_bar_mode_field_defaults_to_false() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"},"appearance":"system"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    assert!(!store.load().menu_bar_mode);
}

#[test]
fn menu_bar_mode_round_trips_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument::new(
        10,
        PersistedTarget::Display,
        "system".into(),
        None,
        "nothing".into(),
        Hotkeys::default(),
        true,
        true,
    );

    store.save(&document).unwrap();

    assert!(store.load().menu_bar_mode);
}
