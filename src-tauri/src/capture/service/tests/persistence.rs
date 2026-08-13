use super::*;
use crate::capture::service::persistence;

fn service_with_startup_target(target: PersistedTarget) -> CaptureService {
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
        startup_target: target,
        appearance: RwLock::new("system".into()),
        diagnostics: DiagnosticLog::disabled(),
        user_paused: AtomicBool::new(false),
    }))
}

#[test]
fn unresolvable_persisted_window_falls_back_to_default_display_with_notice() {
    let service = service_with_startup_target(PersistedTarget::Window {
        bundle_id: "com.other.app".into(),
        title: "A window that has since closed".into(),
    });

    let snapshot = persistence::start_from_persisted(&service).unwrap();

    assert_eq!(snapshot.capture, CaptureState::Capturing);
    assert_eq!(
        snapshot.source.map(|source| source.id),
        Some("display:1".into())
    );
    assert_eq!(
        snapshot.source_notice.as_deref(),
        Some("persisted_window_unavailable")
    );
}

#[test]
fn resolvable_persisted_window_restores_without_a_notice() {
    let service = service_with_startup_target(PersistedTarget::Window {
        bundle_id: "com.example.app".into(),
        title: "window:2".into(),
    });

    let snapshot = persistence::start_from_persisted(&service).unwrap();

    assert_eq!(
        snapshot.source.map(|source| source.id),
        Some("window:2".into())
    );
    assert_eq!(snapshot.source_notice, None);
}

#[test]
fn repicking_a_source_clears_the_fallback_notice() {
    let service = service_with_startup_target(PersistedTarget::Window {
        bundle_id: "com.other.app".into(),
        title: "A window that has since closed".into(),
    });
    let snapshot = persistence::start_from_persisted(&service).unwrap();
    assert!(snapshot.source_notice.is_some());

    let repicked = service.switch_by_id("window:2").unwrap();

    // Once the user picks a live source the fallback story is over; keeping
    // the notice would describe a target situation that no longer exists.
    assert_eq!(repicked.source_notice, None);
}

#[test]
fn switching_source_persists_it_for_the_next_launch() {
    let service = service_with_startup_target(PersistedTarget::Display);

    service.switch_by_id("window:2").unwrap();

    let reloaded = service.0.settings.load();
    assert_eq!(
        reloaded.target,
        PersistedTarget::Window {
            bundle_id: "com.example.app".into(),
            title: "window:2".into(),
        }
    );
}
