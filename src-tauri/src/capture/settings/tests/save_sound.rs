use super::*;

/// The chime is on for fresh installs and for every settings file written
/// before PP-03 existed — the one case a plain `#[serde(default)]` bool
/// would get backwards.
#[test]
fn missing_save_sound_field_defaults_to_on() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"},"appearance":"system"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    assert!(store.load().save_sound);
}

#[test]
fn a_corrupt_file_still_lands_on_the_sound_being_on() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(&path, b"{ not json at all").unwrap();
    let store = SettingsStore::new(path);

    assert!(store.load().save_sound);
}

#[test]
fn save_sound_off_round_trips_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument::new(
        10,
        PersistedTarget::Display,
        "system".into(),
        None,
        "preview".into(),
        Hotkeys::default(),
        false,
        false,
    );

    store.save(&document).unwrap();

    let reloaded = store.load();
    assert!(!reloaded.save_sound);
    assert_eq!(reloaded.after_save, "preview");
    assert_eq!(reloaded.retention_minutes, 10);
}
