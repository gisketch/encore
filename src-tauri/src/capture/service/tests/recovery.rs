use super::*;
use crate::capture::model::RETRY_DELAYS;
use crate::capture::service::recovery;
use std::{thread, time::Duration};

fn fake_backend() -> Box<dyn CaptureBackend> {
    Box::new(FakeBackend {
        fail_window: Arc::new(AtomicBool::new(false)),
        resize_count: Arc::new(AtomicUsize::new(0)),
        ready_status: SCFrameStatus::Complete,
    })
}

/// Backend whose `start` fails a caller-controlled number of times before
/// succeeding, so the bounded retry loop can be driven deterministically
/// without a real ScreenCaptureKit stream.
struct FlakyBackend {
    remaining_failures: Arc<AtomicUsize>,
}

impl CaptureBackend for FlakyBackend {
    fn list_sources(&self) -> Result<Vec<CaptureSource>, &'static str> {
        Ok(vec![source("display:1", SourceKind::Display, true)])
    }

    fn main_display_source(&self) -> Result<CaptureSource, &'static str> {
        Ok(source("display:1", SourceKind::Display, true))
    }

    fn source_exists(&self, source_id: &str) -> bool {
        source_id == "display:1"
    }

    fn start(
        &self,
        source: &CaptureSource,
        _frames: LatestSender<NativeFrame>,
        ready: Sender<CaptureReady>,
        _signals: Sender<RuntimeSignal>,
    ) -> Result<Box<dyn RunningCapture>, &'static str> {
        if self.remaining_failures.load(Ordering::SeqCst) > 0 {
            self.remaining_failures.fetch_sub(1, Ordering::SeqCst);
            return Err("fake_switch_failure");
        }
        let _ = ready.send(CaptureReady {
            source_id: source.id.clone(),
            width: source.width,
            height: source.height,
            status: SCFrameStatus::Complete,
        });
        Ok(Box::new(FakeStream {
            resize_count: Arc::new(AtomicUsize::new(0)),
        }))
    }
}

#[test]
fn stream_error_transitions_capture_to_recovering_before_retrying() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();
    let source_id = service.snapshot().source.unwrap().id;

    let recovering = service.clone();
    let handle = thread::spawn(move || recovery::recover_transient(&recovering, &source_id));

    // The retry loop's first delay is RETRY_DELAYS[0] (>= 1s); this window
    // is well inside it, so the state is observed mid-recovery rather than
    // racing the eventual successful restart.
    thread::sleep(Duration::from_millis(200));
    assert_eq!(service.snapshot().capture, CaptureState::Recovering);
    assert_eq!(
        service.snapshot().error_code.as_deref(),
        Some("transient_stream_failure")
    );

    handle.join().unwrap();
    let snapshot = service.snapshot();
    assert_eq!(snapshot.capture, CaptureState::Capturing);
    assert_eq!(
        snapshot.source.map(|source| source.id),
        Some("display:1".into())
    );
}

#[test]
fn clock_discontinuity_recovers_through_the_same_bounded_path_into_the_same_target() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.start_default().unwrap();
    let source_id = service.snapshot().source.unwrap().id;

    // No stream error at all — only a clock-discontinuity signal, the case
    // where sleep/wake surfaces as a timestamp gap rather than a
    // ScreenCaptureKit error.
    recovery::recover_from_clock_discontinuity(&service, &source_id);

    let snapshot = service.snapshot();
    assert_eq!(snapshot.capture, CaptureState::Capturing);
    assert_eq!(
        snapshot.source.map(|source| source.id),
        Some("display:1".into())
    );
    assert_eq!(snapshot.retry_count, 0);
    assert_eq!(snapshot.error_code, None);
}

#[test]
fn bounded_retry_exhaustion_lands_in_failed_with_a_stable_code() {
    let remaining_failures = Arc::new(AtomicUsize::new(0));
    let (service, _encoder_controls) = service_with_backend_and_controls(Box::new(FlakyBackend {
        remaining_failures: remaining_failures.clone(),
    }));
    service.start_default().unwrap();
    let source_id = service.snapshot().source.unwrap().id;
    // More failures than the retry schedule has attempts for, so every
    // bounded retry is exhausted.
    remaining_failures.store(RETRY_DELAYS.len() + 5, Ordering::SeqCst);

    recovery::recover_transient(&service, &source_id);

    let snapshot = service.snapshot();
    assert_eq!(snapshot.capture, CaptureState::Failed);
    assert_eq!(snapshot.error_code.as_deref(), Some("retry_exhausted"));
    assert_eq!(snapshot.retry_count, RETRY_DELAYS.len() as u8);
}

#[test]
fn source_loss_marks_unavailable_with_a_stable_code_and_keeps_the_original_target() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.switch_by_id("window:2").unwrap();

    service.mark_unavailable("window:2");

    let snapshot = service.snapshot();
    assert_eq!(snapshot.capture, CaptureState::SourceUnavailable);
    assert_eq!(snapshot.error_code.as_deref(), Some("source_unavailable"));
    // No silent target switch: the snapshot still names the lost target
    // rather than quietly reassigning `source` to anything else.
    assert_eq!(
        snapshot.source.map(|source| source.id),
        Some("window:2".into())
    );
}

#[test]
fn source_unavailable_state_supports_retry_repick_and_full_screen_fallback() {
    let (service, _encoder_controls) = service_with_backend_and_controls(fake_backend());
    service.switch_by_id("window:2").unwrap();
    service.mark_unavailable("window:2");
    assert_eq!(service.snapshot().capture, CaptureState::SourceUnavailable);

    // Action 1: retry the same target explicitly.
    let retried = service.retry().unwrap();
    assert_eq!(retried.capture, CaptureState::Capturing);
    assert_eq!(
        retried.source.map(|source| source.id),
        Some("window:2".into())
    );

    service.mark_unavailable("window:2");

    // Action 2: re-pick a different source explicitly (never silent).
    let repicked = service.switch_by_id("display:1").unwrap();
    assert_eq!(repicked.capture, CaptureState::Capturing);
    assert_eq!(
        repicked.source.map(|source| source.id),
        Some("display:1".into())
    );

    service.mark_unavailable("display:1");

    // Action 3: one-click fall back to full screen.
    let fallback = service.start_default().unwrap();
    assert_eq!(fallback.capture, CaptureState::Capturing);
    assert_eq!(
        fallback.source.map(|source| source.id),
        Some("display:1".into())
    );
}
