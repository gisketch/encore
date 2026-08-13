use super::*;

#[test]
fn save_destination_and_after_save_round_trip_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument::new(
        10,
        PersistedTarget::Display,
        "system".into(),
        Some(PathBuf::from("/Users/example/Movies/Custom")),
        "reveal".into(),
        Hotkeys::default(),
        false,
    );

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
}

#[test]
fn missing_save_destination_and_after_save_default_to_the_default_folder_and_nothing() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"},"appearance":"system"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    let loaded = store.load();

    assert_eq!(loaded.save_destination, None);
    assert_eq!(loaded.after_save, "nothing");
}

#[test]
fn invalid_after_save_falls_back_to_the_default_without_losing_other_fields() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":5,"target":{"kind":"display"},"appearance":"dark","after_save":"delete"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    let loaded = store.load();
    assert_eq!(loaded.after_save, "nothing");
    assert_eq!(loaded.appearance, "dark");
}

#[test]
fn open_editor_after_save_round_trips_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument::new(
        10,
        PersistedTarget::Display,
        "system".into(),
        None,
        "open_editor".into(),
        Hotkeys::default(),
        false,
    );

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
}

#[test]
fn valid_after_save_accepts_reveal_nothing_and_open_editor() {
    assert!(valid_after_save("reveal"));
    assert!(valid_after_save("nothing"));
    assert!(valid_after_save("open_editor"));
    assert!(!valid_after_save("delete"));
    assert!(!valid_after_save(""));
}
