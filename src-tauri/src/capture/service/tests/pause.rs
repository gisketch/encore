use super::*;
use crate::capture::service::recovery;
use crate::retention::RollingStore;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

fn scratch_rolling_store(minutes: u8) -> (std::path::PathBuf, RollingStore) {
    let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "encore-capture-pause-test-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let store = RollingStore::open(directory.clone(), minutes).unwrap();
    (directory, store)
}

fn segment(
    directory: &std::path::Path,
    time_ms: u64,
    sequence: u64,
) -> crate::capture::SegmentRecord {
    let path = directory.join(format!("segment-{time_ms}-{sequence:06}.mp4"));
    std::fs::write(&path, [1_u8; 16]).unwrap();
    crate::capture::SegmentRecord {
        path: path.to_string_lossy().into_owned(),
        source_id: "display:1".into(),
        width: 1920,
        height: 1080,
        byte_size: 16,
        duration_micros: 10_000_000,
        session_start_micros: time_ms * 1_000,
        session_end_micros: (time_ms + 10_000) * 1_000,
        completed_at_unix_ms: time_ms + 10_000,
        boundary: crate::capture::SegmentBoundary::Duration,
    }
}

fn fake_backend() -> Box<dyn CaptureBackend> {
    Box::new(FakeBackend {
        fail_window: Arc::new(AtomicBool::new(false)),
        resize_count: Arc::new(AtomicUsize::new(0)),
        ready_status: SCFrameStatus::Complete,
    })
}

#[test]
fn pause_requires_active_capture() {
    let service = service_with_backend(fake_backend());

    assert_eq!(service.pause(), Err("pause_requires_active_capture".into()));
    assert_eq!(service.snapshot().capture, CaptureState::Stopped);
}

#[test]
fn pause_stops_the_stream_and_tells_the_encoder_to_finalize() {
    let (service, encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();
    service.update(|snapshot| snapshot.pipeline = PipelineState::Encoding);

    let snapshot = service.pause().unwrap();

    assert_eq!(snapshot.capture, CaptureState::Paused);
    assert_eq!(snapshot.pipeline, PipelineState::Idle);
    assert_eq!(
        snapshot.source.map(|source| source.id),
        Some("display:1".into())
    );
    assert_eq!(encoder_controls.try_recv(), Ok(EncoderCommand::Stop));
    assert!(service
        .0
        .stream
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_none());
}

#[test]
fn resume_requires_paused_capture() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();

    assert_eq!(
        service.resume(),
        Err("resume_requires_paused_capture".into())
    );
}

#[test]
fn resume_restarts_capture_into_the_same_target() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();
    service.pause().unwrap();

    let snapshot = service.resume().unwrap();

    assert_eq!(snapshot.capture, CaptureState::Capturing);
    assert_eq!(
        snapshot.source.map(|source| source.id),
        Some("display:1".into())
    );
}

#[test]
fn a_queued_stream_error_cannot_override_an_explicit_pause() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();
    service.pause().unwrap();

    // A TransientFailure signal queued before (or raised by) the pause's own
    // stream teardown arrives after the user paused: recovery must bail
    // instead of restarting capture behind the user's back.
    recovery::recover_transient(&service, "display:1");

    assert_eq!(service.snapshot().capture, CaptureState::Paused);
    assert!(service
        .0
        .stream
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_none());
}

#[test]
fn a_stale_healthy_signal_cannot_unpause_without_a_stream() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();
    service.pause().unwrap();

    // A Healthy signal queued just before the pause must not flip the state
    // to Capturing: the stream is gone, so that claim would be a lie.
    service.mark_healthy("display:1");

    assert_eq!(service.snapshot().capture, CaptureState::Paused);
}

#[test]
fn resume_clears_the_pause_guard_so_recovery_works_again() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();
    service.pause().unwrap();
    service.resume().unwrap();

    recovery::recover_transient(&service, "display:1");

    assert_eq!(service.snapshot().capture, CaptureState::Capturing);
}

#[test]
fn pausing_freezes_retention_and_paused_evidence_stays_replay_triggerable() {
    let (directory, rolling) = scratch_rolling_store(5);
    for sequence in 0..=2 {
        rolling
            .admit(&segment(&directory, sequence * 10_000, sequence))
            .unwrap();
    }
    let (service, _encoder_controls) = service_with_rolling(fake_backend(), rolling.clone());
    service.start_default().unwrap();
    let before = rolling.snapshot();

    service.pause().unwrap();

    // No new segments are admitted while paused, so an explicit prune (what
    // any future wall-clock trigger would call) leaves retained evidence
    // untouched instead of aging it out.
    let after_prune = rolling.prune().unwrap();
    assert_eq!(after_prune, before);
    assert_eq!(service.snapshot().retention, before);

    // Replay-trigger eligibility follows retained evidence, not capture
    // focus: leasing the paused store's segments still succeeds.
    let lease = service.rolling_store().lease().unwrap();
    assert_eq!(lease.segments().len(), 3);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn resume_admits_new_segments_and_restores_normal_pruning() {
    let (directory, rolling) = scratch_rolling_store(5);
    for sequence in 0..=2 {
        rolling
            .admit(&segment(&directory, sequence * 10_000, sequence))
            .unwrap();
    }
    let (service, _encoder_controls) = service_with_rolling(fake_backend(), rolling);
    service.start_default().unwrap();
    service.pause().unwrap();
    service.resume().unwrap();

    for sequence in 3..=40 {
        service.record_segment(segment(&directory, sequence * 10_000, sequence));
    }

    assert_eq!(service.snapshot().retention.retained_segments, 30);

    let _ = std::fs::remove_dir_all(&directory);
}
