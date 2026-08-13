use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("encore-settings-test-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn window_source(bundle_id: &str, title: &str) -> CaptureSource {
    CaptureSource {
        id: "window:9".into(),
        kind: SourceKind::Window,
        label: format!("App — {title}"),
        width: 800,
        height: 600,
        is_main: false,
        bundle_id: Some(bundle_id.into()),
        title: Some(title.into()),
    }
}

fn display_source() -> CaptureSource {
    CaptureSource {
        id: "display:1".into(),
        kind: SourceKind::Display,
        label: "Main display".into(),
        width: 1920,
        height: 1080,
        is_main: true,
        bundle_id: None,
        title: None,
    }
}

#[test]
fn settings_round_trip_through_atomic_write() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));
    let document = SettingsDocument {
        version: CURRENT_VERSION,
        retention_minutes: 5,
        target: PersistedTarget::Window {
            bundle_id: "com.example.app".into(),
            title: "Notes".into(),
        },
        appearance: "dark".into(),
    };

    store.save(&document).unwrap();

    assert_eq!(store.load(), document);
}

#[test]
fn atomic_write_leaves_no_temporary_file_behind() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("settings.json"));

    store.save(&SettingsDocument::default()).unwrap();

    let leftovers = fs::read_dir(directory.path())
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().contains(".tmp"));
    assert!(!leftovers);
}

#[test]
fn missing_file_yields_defaults_and_does_not_error() {
    let directory = TestDirectory::new();
    let store = SettingsStore::new(directory.path().join("missing.json"));

    assert_eq!(store.load(), SettingsDocument::default());
}

#[test]
fn corrupt_file_yields_defaults_without_blocking_startup() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(&path, b"{ this is not valid json").unwrap();
    let store = SettingsStore::new(path);

    assert_eq!(store.load(), SettingsDocument::default());
}

#[test]
fn out_of_range_retention_falls_back_to_the_default() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":7,"target":{"kind":"display"}}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    assert_eq!(store.load().retention_minutes, DEFAULT_RETENTION_MINUTES);
}

#[test]
fn unknown_fields_are_ignored_rather_than_fatal() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"version":1,"retention_minutes":5,"target":{"kind":"display"},"future_field":"x"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);

    let loaded = store.load();
    assert_eq!(loaded.retention_minutes, 5);
    assert_eq!(loaded.target, PersistedTarget::Display);
}

#[test]
fn window_target_resolves_by_bundle_and_title() {
    let sources = vec![display_source(), window_source("com.example.app", "Notes")];
    let target = PersistedTarget::Window {
        bundle_id: "com.example.app".into(),
        title: "Notes".into(),
    };

    let resolved = resolve_target(&sources, &target);

    assert_eq!(resolved.map(|source| source.id), Some("window:9".into()));
}

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
    let document = SettingsDocument::new(10, PersistedTarget::Display, "light".into());

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

#[test]
fn unresolvable_window_target_falls_back_to_default_display() {
    let sources = vec![display_source(), window_source("com.example.app", "Notes")];
    let target = PersistedTarget::Window {
        bundle_id: "com.example.app".into(),
        title: "A window that has since closed".into(),
    };

    assert_eq!(resolve_target(&sources, &target), None);
}

#[test]
fn display_target_never_resolves_to_a_specific_source() {
    let sources = vec![display_source()];

    assert_eq!(resolve_target(&sources, &PersistedTarget::Display), None);
}
