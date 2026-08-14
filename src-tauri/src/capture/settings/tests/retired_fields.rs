use super::*;

/// MB-02 removed the `menu_bar_mode` field: the menu bar became Encore's
/// permanent control surface rather than a mode. Retiring a persisted
/// field has to be tolerant, not destructive — a settings file written by
/// an older build still records it, and loading such a file must keep every
/// surviving value rather than falling back to defaults.
#[test]
fn a_document_recording_the_retired_menu_bar_field_still_loads_intact() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":5,"target":{"kind":"display"},
             "appearance":"dark","after_save":"reveal","menu_bar_mode":true,
             "save_sound":false}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    let document = store.load();

    assert_eq!(document.retention_minutes, 5);
    assert_eq!(document.appearance, "dark");
    assert_eq!(document.after_save, "reveal");
    assert!(!document.save_sound);
}

/// The retired field must not survive a rewrite either: once this build
/// persists anything, the stale key is gone rather than being carried
/// forward forever.
#[test]
fn rewriting_a_migrated_document_drops_the_retired_field() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":10,"target":{"kind":"display"},
             "menu_bar_mode":true}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path.clone());

    store.save(&store.load()).unwrap();

    let written = fs::read_to_string(&path).unwrap();
    assert!(
        !written.contains("menu_bar_mode"),
        "retired field survived a rewrite: {written}"
    );
}
