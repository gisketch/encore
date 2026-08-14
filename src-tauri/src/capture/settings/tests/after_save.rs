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
        true,
    );

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
}

#[test]
fn missing_save_destination_and_after_save_default_to_the_default_folder_and_preview() {
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
    assert_eq!(loaded.after_save, "preview");
}

/// Changing the default (PP-01) must not touch installations that already
/// chose something: `nothing` was the previous default and is still a
/// valid value, so a document that records it loads it back verbatim.
#[test]
fn an_already_persisted_choice_survives_the_default_change_untouched() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"},"appearance":"system","after_save":"nothing"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    assert_eq!(store.load().after_save, "nothing");
}

#[test]
fn preview_after_save_round_trips_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument::new(
        10,
        PersistedTarget::Display,
        "system".into(),
        None,
        "preview".into(),
        Hotkeys::default(),
        true,
    );

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
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
    assert_eq!(loaded.after_save, "preview");
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
        true,
    );

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
}

#[test]
fn valid_after_save_accepts_preview_reveal_nothing_and_open_editor() {
    assert!(valid_after_save("preview"));
    assert!(valid_after_save("reveal"));
    assert!(valid_after_save("nothing"));
    assert!(valid_after_save("open_editor"));
    assert!(!valid_after_save("delete"));
    assert!(!valid_after_save(""));
}
