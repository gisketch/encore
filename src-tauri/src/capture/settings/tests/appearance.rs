use super::*;

#[test]
fn missing_appearance_field_defaults_to_system() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"}}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    assert_eq!(store.load().appearance, DEFAULT_APPEARANCE);
}

#[test]
fn invalid_appearance_falls_back_to_the_default_without_losing_other_fields() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":5,"target":{"kind":"display"},"appearance":"sepia"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    let loaded = store.load();
    assert_eq!(loaded.appearance, DEFAULT_APPEARANCE);
    assert_eq!(loaded.retention_minutes, 5);
}

#[test]
fn appearance_round_trips_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument::new(
        10,
        PersistedTarget::Display,
        "light".into(),
        None,
        "nothing".into(),
        Hotkeys::default(),
        false,
    );

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
}

#[test]
fn valid_appearance_accepts_only_the_three_known_choices() {
    assert!(valid_appearance("light"));
    assert!(valid_appearance("dark"));
    assert!(valid_appearance("system"));
    assert!(!valid_appearance("sepia"));
    assert!(!valid_appearance(""));
}
