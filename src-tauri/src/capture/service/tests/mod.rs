mod concurrency;
mod diagnostics;
mod pause;
mod persistence;
mod recovery;

use super::*;
use crate::capture::{
    backend::CaptureBackend,
    model::{PipelineState, SourceKind},
    permission::PermissionProvider,
    platform::CaptureReady,
    settings::{PersistedTarget, SettingsStore},
};
use crate::diagnostics::DiagnosticLog;
use crate::encoder::EncoderCommand;
use crate::retention::RollingStore;
use crossbeam_channel::Receiver;
use screencapturekit::cm::SCFrameStatus;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

static NEXT_SETTINGS_FILE: AtomicU64 = AtomicU64::new(1);

fn scratch_settings_store() -> SettingsStore {
    let id = NEXT_SETTINGS_FILE.fetch_add(1, Ordering::Relaxed);
    SettingsStore::new(std::env::temp_dir().join(format!(
        "encore-service-test-settings-{}-{id}.json",
        std::process::id()
    )))
}

struct GrantedPermission;

impl PermissionProvider for GrantedPermission {
    fn current(&self) -> PermissionState {
        PermissionState::Granted
    }

    fn request(&self) -> PermissionState {
        PermissionState::Granted
    }
}

struct FakeStream {
    resize_count: Arc<AtomicUsize>,
}

impl RunningCapture for FakeStream {
    fn commit(&self) {}

    fn resize(&self, _width: u32, _height: u32) -> Result<(), &'static str> {
        self.resize_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

struct FakeBackend {
    fail_window: Arc<AtomicBool>,
    resize_count: Arc<AtomicUsize>,
    ready_status: SCFrameStatus,
}

impl CaptureBackend for FakeBackend {
    fn list_sources(&self) -> Result<Vec<CaptureSource>, &'static str> {
        Ok(vec![
            source("display:1", SourceKind::Display, true),
            source("window:2", SourceKind::Window, false),
        ])
    }

    fn main_display_source(&self) -> Result<CaptureSource, &'static str> {
        Ok(source("display:1", SourceKind::Display, true))
    }

    fn source_exists(&self, source_id: &str) -> bool {
        ["display:1", "window:2"].contains(&source_id)
    }

    fn start(
        &self,
        source: &CaptureSource,
        _frames: LatestSender<NativeFrame>,
        ready: Sender<CaptureReady>,
        _signals: Sender<RuntimeSignal>,
    ) -> Result<Box<dyn RunningCapture>, &'static str> {
        if source.kind == SourceKind::Window && self.fail_window.load(Ordering::Relaxed) {
            return Err("fake_switch_failure");
        }
        let _ = ready.send(CaptureReady {
            source_id: source.id.clone(),
            width: source.width,
            height: source.height,
            status: self.ready_status,
        });
        Ok(Box::new(FakeStream {
            resize_count: self.resize_count.clone(),
        }))
    }
}

fn source(id: &str, kind: SourceKind, is_main: bool) -> CaptureSource {
    CaptureSource {
        id: id.into(),
        kind,
        label: id.into(),
        width: 1920,
        height: 1080,
        is_main,
        bundle_id: (kind == SourceKind::Window).then(|| "com.example.app".into()),
        title: (kind == SourceKind::Window).then(|| id.into()),
    }
}

fn service_with_backend(backend: Box<dyn CaptureBackend>) -> CaptureService {
    service_with_backend_and_controls(backend).0
}

fn service_with_backend_and_controls(
    backend: Box<dyn CaptureBackend>,
) -> (CaptureService, Receiver<EncoderCommand>) {
    service_with_rolling(backend, RollingStore::empty_for_tests())
}

fn service_with_rolling(
    backend: Box<dyn CaptureBackend>,
    rolling: RollingStore,
) -> (CaptureService, Receiver<EncoderCommand>) {
    let (frames, _receiver) = latest_channel(3);
    let (signals, _signal_receiver) = bounded(8);
    let (encoder_controls, encoder_receiver) = bounded(4);
    (
        CaptureService(Arc::new(Inner {
            app: None,
            snapshot: RwLock::new(CaptureSnapshot {
                permission: PermissionState::Granted,
                retention: rolling.snapshot(),
                ..CaptureSnapshot::default()
            }),
            operation: Mutex::new(()),
            stream: Mutex::new(None),
            permission: Box::new(GrantedPermission),
            backend,
            frames,
            signals,
            encoder_controls,
            rolling,
            settings: scratch_settings_store(),
            startup_target: PersistedTarget::Display,
            appearance: RwLock::new("system".into()),
            diagnostics: DiagnosticLog::disabled(),
            user_paused: AtomicBool::new(false),
        })),
        encoder_receiver,
    )
}

/// Builds a service wired to a real (file-backed) diagnostic log instead of
/// the disabled handle every other test helper uses, for tests that assert
/// on the log's contents rather than just the in-memory snapshot.
fn service_with_diagnostics(
    backend: Box<dyn CaptureBackend>,
    diagnostics: DiagnosticLog,
) -> CaptureService {
    let (frames, _receiver) = latest_channel(3);
    let (signals, _signal_receiver) = bounded(8);
    let (encoder_controls, _encoder_receiver) = bounded(4);
    let rolling = RollingStore::empty_for_tests();
    CaptureService(Arc::new(Inner {
        app: None,
        snapshot: RwLock::new(CaptureSnapshot {
            permission: PermissionState::Granted,
            retention: rolling.snapshot(),
            ..CaptureSnapshot::default()
        }),
        operation: Mutex::new(()),
        stream: Mutex::new(None),
        permission: Box::new(GrantedPermission),
        backend,
        frames,
        signals,
        encoder_controls,
        rolling,
        settings: scratch_settings_store(),
        startup_target: PersistedTarget::Display,
        appearance: RwLock::new("system".into()),
        diagnostics,
        user_paused: AtomicBool::new(false),
    }))
}

#[test]
fn retention_duration_rejects_values_outside_five_and_ten() {
    let service = test_service(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicUsize::new(0)),
    );

    assert_eq!(
        service.set_retention_minutes(5).unwrap().retention.minutes,
        5
    );
    assert_eq!(
        service.set_retention_minutes(7),
        Err("retention_duration_invalid".into())
    );
    assert_eq!(service.snapshot().retention.minutes, 5);
}

#[test]
fn failed_segment_admission_halts_encoding_without_stopping_capture() {
    let (service, encoder_controls) = service_with_backend_and_controls(Box::new(FakeBackend {
        fail_window: Arc::new(AtomicBool::new(false)),
        resize_count: Arc::new(AtomicUsize::new(0)),
        ready_status: SCFrameStatus::Complete,
    }));
    service.start_default().unwrap();
    service.record_segment(crate::capture::SegmentRecord {
        path: "/not-inside-the-rolling-store/segment-1-000001.mp4".into(),
        source_id: "display:1".into(),
        width: 1920,
        height: 1080,
        byte_size: 1,
        duration_micros: 10_000_000,
        session_start_micros: 0,
        session_end_micros: 10_000_000,
        completed_at_unix_ms: 10_000,
        boundary: crate::capture::SegmentBoundary::Duration,
    });

    let snapshot = service.snapshot();
    assert_eq!(snapshot.capture, CaptureState::Capturing);
    assert_eq!(snapshot.pipeline, PipelineState::Failed);
    assert_eq!(
        snapshot.retention.error_code.as_deref(),
        Some("retention_segment_invalid")
    );
    assert_eq!(encoder_controls.try_recv(), Ok(EncoderCommand::Halt));
}

fn test_service(fail_window: Arc<AtomicBool>, resize_count: Arc<AtomicUsize>) -> CaptureService {
    service_with_backend(Box::new(FakeBackend {
        fail_window,
        resize_count,
        ready_status: SCFrameStatus::Complete,
    }))
}

#[test]
fn failed_replacement_keeps_the_active_source() {
    let fail_window = Arc::new(AtomicBool::new(false));
    let service = test_service(fail_window.clone(), Arc::new(AtomicUsize::new(0)));
    service.switch_by_id("display:1").unwrap();
    fail_window.store(true, Ordering::Relaxed);

    assert_eq!(
        service.switch_by_id("window:2"),
        Err("fake_switch_failure".into())
    );
    let snapshot = service.snapshot();
    assert_eq!(snapshot.capture, CaptureState::Capturing);
    assert_eq!(snapshot.source.unwrap().id, "display:1");
}

#[test]
fn geometry_change_resizes_without_stopping_capture() {
    let resize_count = Arc::new(AtomicUsize::new(0));
    let service = test_service(Arc::new(AtomicBool::new(false)), resize_count.clone());
    service.switch_by_id("display:1").unwrap();

    resize::handle(&service, "display:1", 1600, 900);

    let snapshot = service.snapshot();
    assert_eq!(snapshot.capture, CaptureState::Capturing);
    assert_eq!(
        snapshot.source.map(|source| (source.width, source.height)),
        Some((1600, 900))
    );
    assert_eq!(resize_count.load(Ordering::Relaxed), 1);
}

#[test]
fn initial_blank_frame_starts_a_paused_stream_instead_of_timing_out() {
    let service = service_with_backend(Box::new(FakeBackend {
        fail_window: Arc::new(AtomicBool::new(false)),
        resize_count: Arc::new(AtomicUsize::new(0)),
        ready_status: SCFrameStatus::Blank,
    }));

    let snapshot = service.start_default().unwrap();

    assert_eq!(snapshot.capture, CaptureState::Paused);
    assert_eq!(snapshot.source.unwrap().id, "display:1");
}

#[test]
fn stopping_capture_tells_the_encoder_to_finalize() {
    let (service, encoder_controls) = service_with_backend_and_controls(Box::new(FakeBackend {
        fail_window: Arc::new(AtomicBool::new(false)),
        resize_count: Arc::new(AtomicUsize::new(0)),
        ready_status: SCFrameStatus::Complete,
    }));
    service.start_default().unwrap();
    service.update(|snapshot| snapshot.pipeline = PipelineState::Encoding);

    let snapshot = service.stop();

    assert_eq!(snapshot.capture, CaptureState::Stopped);
    assert_eq!(snapshot.pipeline, PipelineState::Idle);
    assert_eq!(encoder_controls.try_recv(), Ok(EncoderCommand::Stop));
}
