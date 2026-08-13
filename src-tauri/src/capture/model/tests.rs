use super::*;

#[test]
fn aspect_fit_never_crops_or_upscales() {
    assert_eq!(aspect_fit(3840, 2160), (1920, 1080));
    assert_eq!(aspect_fit(2560, 1080), (1920, 810));
    assert_eq!(aspect_fit(800, 600), (800, 600));
}

#[test]
fn capture_requires_starting_before_capturing() {
    let mut snapshot = CaptureSnapshot::default();
    assert_eq!(
        snapshot.set_capture(CaptureState::Capturing),
        Err("invalid_capture_transition")
    );
    snapshot.set_capture(CaptureState::Starting).unwrap();
    snapshot.set_capture(CaptureState::Capturing).unwrap();
}

#[test]
fn diagnostics_do_not_contain_a_source_label() {
    let snapshot = CaptureSnapshot {
        source: Some(CaptureSource {
            id: "window:42".into(),
            kind: SourceKind::Window,
            label: "Private document title".into(),
            width: 900,
            height: 700,
            is_main: false,
            bundle_id: Some("com.example.app".into()),
            title: Some("Private document title".into()),
        }),
        ..CaptureSnapshot::default()
    };
    let json = serde_json::to_string(&DiagnosticRecord::from(&snapshot)).unwrap();
    assert!(!json.contains("Private document title"));
}

#[test]
fn denial_and_restart_do_not_prompt_loop() {
    assert!(should_request_permission(
        PermissionState::PermissionRequired
    ));
    assert!(!should_request_permission(
        PermissionState::PermissionDenied
    ));
    assert!(!should_request_permission(PermissionState::RestartRequired));
}

#[test]
fn failed_switch_rolls_back_only_when_old_stream_is_active() {
    assert_eq!(
        switch_failure_state(CaptureState::Capturing, true, true),
        CaptureState::Capturing
    );
    assert_eq!(
        switch_failure_state(CaptureState::Paused, true, true),
        CaptureState::Paused
    );
    assert_eq!(
        switch_failure_state(CaptureState::SourceUnavailable, true, false),
        CaptureState::SourceUnavailable
    );
    assert_eq!(
        switch_failure_state(CaptureState::Capturing, true, false),
        CaptureState::SourceUnavailable
    );
    assert_eq!(
        switch_failure_state(CaptureState::Capturing, false, true),
        CaptureState::Failed
    );
    assert_eq!(
        switch_failure_state(CaptureState::Stopped, false, false),
        CaptureState::Failed
    );
}

#[test]
fn pause_unavailable_and_recovery_transitions_are_explicit() {
    let mut snapshot = CaptureSnapshot::default();
    snapshot.set_capture(CaptureState::Starting).unwrap();
    snapshot.set_capture(CaptureState::Capturing).unwrap();
    snapshot.set_capture(CaptureState::Paused).unwrap();
    snapshot.set_capture(CaptureState::Capturing).unwrap();
    snapshot
        .set_capture(CaptureState::SourceUnavailable)
        .unwrap();
    snapshot.set_capture(CaptureState::Starting).unwrap();
    snapshot.set_capture(CaptureState::Failed).unwrap();
    assert_eq!(snapshot.capture, CaptureState::Failed);
    assert_eq!(
        RETRY_DELAYS,
        [
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4)
        ]
    );
}

#[test]
fn late_signal_from_previous_source_is_ignored() {
    let snapshot = CaptureSnapshot {
        source: Some(CaptureSource {
            id: "window:22".into(),
            kind: SourceKind::Window,
            label: "Current".into(),
            width: 800,
            height: 600,
            is_main: false,
            bundle_id: Some("com.example.app".into()),
            title: Some("Current".into()),
        }),
        ..CaptureSnapshot::default()
    };
    assert!(signal_matches_source(&snapshot, "window:22"));
    assert!(!signal_matches_source(&snapshot, "display:1"));
}
