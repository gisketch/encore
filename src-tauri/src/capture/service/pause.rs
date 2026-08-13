use super::CaptureService;
use crate::capture::model::{CaptureSnapshot, CaptureState, PipelineState};
use crate::encoder::EncoderCommand;
use std::sync::atomic::Ordering;

impl CaptureService {
    /// Explicit pause: user or tray intent to stop producing new evidence
    /// while keeping everything retained so far replay-triggerable. Unlike
    /// `stop`, the source is kept so `resume` can restart into the same
    /// target; unlike the monitor's automatic blank/suspended pause, the
    /// stream itself is torn down rather than left running.
    pub fn pause(&self) -> Result<CaptureSnapshot, String> {
        let _operation = self
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.snapshot().capture != CaptureState::Capturing {
            return Err("pause_requires_active_capture".into());
        }
        self.0.user_paused.store(true, Ordering::Release);
        self.update(|snapshot| {
            let _ = snapshot.set_capture(CaptureState::Paused);
            snapshot.error_code = None;
            snapshot.pipeline = PipelineState::Idle;
            snapshot.encoder_error_code = None;
        });
        let stream = self
            .0
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        drop(stream);
        let _ = self.0.encoder_controls.try_send(EncoderCommand::Stop);
        self.log_capture(CaptureState::Paused, None);
        Ok(self.snapshot())
    }

    /// Restarts capture into the target that was active at pause time. This
    /// is the single resume path: AL-03 wake recovery reuses it as-is, with
    /// no UI-specific behavior layered on top.
    pub fn resume(&self) -> Result<CaptureSnapshot, String> {
        let _operation = self
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let snapshot = self.snapshot();
        if snapshot.capture != CaptureState::Paused {
            return Err("resume_requires_paused_capture".into());
        }
        let source = snapshot
            .source
            .ok_or_else(|| "capture_no_target".to_string())?;
        self.switch_source_locked(source)
    }

    /// True between an explicit `pause` and the next successful capture
    /// start. Recovery and healthy-signal handling bail while this holds so
    /// a queued signal can never silently un-pause the user or claim
    /// capture without a stream.
    pub(super) fn is_user_paused(&self) -> bool {
        self.0.user_paused.load(Ordering::Acquire)
    }
}
