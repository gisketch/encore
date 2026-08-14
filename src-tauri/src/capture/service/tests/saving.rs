use super::*;
use std::{fs, path::PathBuf};

static NEXT_DESTINATION: AtomicU64 = AtomicU64::new(1);

/// A scratch default destination unique per test, mirroring
/// `scratch_settings_store`'s per-test isolation.
fn scratch_default_destination() -> PathBuf {
    let id = NEXT_DESTINATION.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "encore-service-test-destination-{}-{id}",
        std::process::id()
    ))
}

fn service_with_default_destination(default_destination: PathBuf) -> CaptureService {
    let (frames, _receiver) = latest_channel(3);
    let (signals, _signal_receiver) = bounded(8);
    let (encoder_controls, _encoder_receiver) = bounded(4);
    CaptureService(Arc::new(Inner {
        app: None,
        snapshot: RwLock::new(CaptureSnapshot {
            permission: PermissionState::Granted,
            ..CaptureSnapshot::default()
        }),
        operation: Mutex::new(()),
        stream: Mutex::new(None),
        permission: Box::new(GrantedPermission),
        backend: Box::new(FakeBackend {
            fail_window: Arc::new(AtomicBool::new(false)),
            resize_count: Arc::new(AtomicUsize::new(0)),
            ready_status: SCFrameStatus::Complete,
        }),
        frames,
        signals,
        encoder_controls,
        rolling: RollingStore::empty_for_tests(),
        settings: scratch_settings_store(),
        default_target: RwLock::new(PersistedTarget::Display),
        appearance: RwLock::new("system".into()),
        save_destination: RwLock::new(None),
        default_destination,
        after_save: RwLock::new("nothing".into()),
        hotkeys: RwLock::new(Hotkeys::default()),
        save_sound: RwLock::new(true),
        diagnostics: DiagnosticLog::disabled(),
        user_paused: AtomicBool::new(false),
    }))
}

#[test]
fn save_destination_defaults_to_none_and_resolves_to_the_default_folder() {
    let default_destination = scratch_default_destination();
    let service = service_with_default_destination(default_destination.clone());

    assert_eq!(service.save_destination(), None);
    assert_eq!(service.resolved_save_destination(), default_destination);
}

#[test]
fn setting_a_save_destination_validates_creates_and_persists_it() {
    let default_destination = scratch_default_destination();
    let service = service_with_default_destination(default_destination);
    let custom = scratch_default_destination().join("custom-replays");

    let snapshot = service.set_save_destination(Some(custom.clone())).unwrap();

    assert!(custom.is_dir());
    assert_eq!(snapshot.save_destination, custom);
    assert_eq!(service.resolved_save_destination(), custom);
    assert_eq!(service.0.settings.load().save_destination, Some(custom));
}

#[test]
fn an_unusable_save_destination_is_rejected_without_touching_the_persisted_value() {
    let default_destination = scratch_default_destination();
    let service = service_with_default_destination(default_destination);
    let blocking_file = scratch_default_destination();
    fs::create_dir_all(blocking_file.parent().unwrap()).unwrap();
    fs::write(&blocking_file, b"not a directory").unwrap();
    let unusable = blocking_file.join("replays");

    let result = service.set_save_destination(Some(unusable));

    assert_eq!(result.unwrap_err(), "export_destination_unavailable");
    assert_eq!(service.save_destination(), None);
    assert_eq!(service.0.settings.load().save_destination, None);
}

#[test]
fn save_destination_update_preserves_other_persisted_fields() {
    let default_destination = scratch_default_destination();
    let service = service_with_default_destination(default_destination);
    service.set_retention_minutes(5).unwrap();
    let custom = scratch_default_destination();

    service.set_save_destination(Some(custom)).unwrap();

    let reloaded = service.0.settings.load();
    assert_eq!(reloaded.retention_minutes, 5);
    assert_eq!(reloaded.appearance, "system");
}

#[test]
fn after_save_round_trips_through_the_setter() {
    let service = service_with_default_destination(scratch_default_destination());
    assert_eq!(service.after_save(), "nothing");

    let snapshot = service.set_after_save("reveal".into()).unwrap();

    assert_eq!(snapshot.after_save, "reveal");
    assert_eq!(service.after_save(), "reveal");
    assert_eq!(service.0.settings.load().after_save, "reveal");
}

#[test]
fn an_invalid_after_save_value_is_rejected_without_touching_the_persisted_value() {
    let service = service_with_default_destination(scratch_default_destination());
    // Persist a real choice first, so `load()` reads a written document
    // rather than the fresh-install default.
    service.set_after_save("nothing".into()).unwrap();

    let result = service.set_after_save("delete".into());

    assert_eq!(result.unwrap_err(), "after_save_invalid");
    assert_eq!(service.after_save(), "nothing");
    assert_eq!(service.0.settings.load().after_save, "nothing");
}

#[test]
fn after_save_accepts_open_editor_and_round_trips_through_the_setter() {
    let service = service_with_default_destination(scratch_default_destination());

    let snapshot = service.set_after_save("open_editor".into()).unwrap();

    assert_eq!(snapshot.after_save, "open_editor");
    assert_eq!(service.after_save(), "open_editor");
    assert_eq!(service.0.settings.load().after_save, "open_editor");
}

#[test]
fn after_save_accepts_preview_and_round_trips_through_the_setter() {
    let service = service_with_default_destination(scratch_default_destination());

    let snapshot = service.set_after_save("preview".into()).unwrap();

    assert_eq!(snapshot.after_save, "preview");
    assert_eq!(service.after_save(), "preview");
    assert_eq!(service.0.settings.load().after_save, "preview");
}

#[test]
fn save_sound_defaults_to_on_and_round_trips_through_the_setter() {
    let service = service_with_default_destination(scratch_default_destination());
    assert!(service.save_sound());

    let snapshot = service.set_save_sound(false).unwrap();

    assert!(!snapshot.save_sound);
    assert!(!service.save_sound());
    assert!(!service.0.settings.load().save_sound);
}

#[test]
fn save_sound_update_preserves_other_persisted_fields() {
    let service = service_with_default_destination(scratch_default_destination());
    service.set_retention_minutes(5).unwrap();
    service.set_after_save("reveal".into()).unwrap();

    service.set_save_sound(false).unwrap();

    let reloaded = service.0.settings.load();
    assert_eq!(reloaded.retention_minutes, 5);
    assert_eq!(reloaded.after_save, "reveal");
    assert!(!reloaded.save_sound);
}
