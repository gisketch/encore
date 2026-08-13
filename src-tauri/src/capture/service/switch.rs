use super::CaptureService;
use crate::capture::model::{switch_failure_state, CaptureSnapshot, CaptureSource, CaptureState};
use crossbeam_channel::bounded;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Source-switch mechanics shared by every capture entry point (start,
/// switch, retry, resume, and every bounded-retry attempt): acquire the
/// operation lock, start the candidate stream, wait for its first frame, and
/// either commit it or roll back to a typed failure state. Split out of
/// `service.rs` to keep that file under the repo's size-smell threshold.
impl CaptureService {
    pub(super) fn switch_source(&self, source: CaptureSource) -> Result<CaptureSnapshot, String> {
        let _operation = self
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.switch_source_locked(source)
    }

    pub(super) fn switch_source_locked(
        &self,
        source: CaptureSource,
    ) -> Result<CaptureSnapshot, String> {
        let rollback_state = self.snapshot().capture;
        let had_active_stream = self
            .0
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some();
        self.update(|snapshot| {
            let _ = snapshot.set_capture(CaptureState::Starting);
            snapshot.error_code = None;
        });
        let (ready_sender, ready_receiver) = bounded(1);
        let candidate = self
            .0
            .backend
            .start(
                &source,
                self.0.frames.clone(),
                ready_sender,
                self.0.signals.clone(),
            )
            .map_err(|code| self.switch_failed(code, rollback_state))?;

        let deadline = Instant::now() + Duration::from_secs(3);
        let ready = loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let received = match ready_receiver.recv_timeout(remaining) {
                Ok(ready) => ready,
                Err(_) => {
                    drop(candidate);
                    return Err(self.switch_failed("first_frame_timeout", rollback_state));
                }
            };
            let is_healthy = matches!(
                received.status,
                screencapturekit::cm::SCFrameStatus::Complete
                    | screencapturekit::cm::SCFrameStatus::Started
                    | screencapturekit::cm::SCFrameStatus::Idle
            );
            let is_initial_pause = !had_active_stream
                && matches!(
                    received.status,
                    screencapturekit::cm::SCFrameStatus::Blank
                        | screencapturekit::cm::SCFrameStatus::Suspended
                );
            if is_healthy || is_initial_pause {
                break received;
            }
        };
        if ready.source_id != source.id {
            drop(candidate);
            return Err(self.switch_failed("source_identity_mismatch", rollback_state));
        }
        let mut source = source;
        source.width = ready.width;
        source.height = ready.height;

        candidate.commit();

        let old_stream = self
            .0
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .replace(candidate);
        let next = match ready.status {
            screencapturekit::cm::SCFrameStatus::Blank
            | screencapturekit::cm::SCFrameStatus::Suspended => CaptureState::Paused,
            _ => CaptureState::Capturing,
        };
        self.update(|snapshot| {
            snapshot.source = Some(source);
            snapshot.retry_count = 0;
            snapshot.error_code = None;
            snapshot.source_notice = None;
            let _ = snapshot.set_capture(next);
        });
        self.0.user_paused.store(false, Ordering::Release);
        drop(old_stream);
        self.log_capture(next, None);
        Ok(self.snapshot())
    }

    pub(super) fn switch_failed(&self, code: &'static str, rollback_state: CaptureState) -> String {
        let had_stream = self
            .0
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some();
        let previous_source_available = self
            .snapshot()
            .source
            .as_ref()
            .map(|source| self.0.backend.source_exists(&source.id))
            .unwrap_or(false);
        let fallback = switch_failure_state(rollback_state, had_stream, previous_source_available);
        self.update(|snapshot| {
            snapshot.error_code = Some(code.into());
            let _ = snapshot.set_capture(fallback);
        });
        self.log_capture(fallback, Some(code.into()));
        code.into()
    }
}
