use super::{recovery, resize, CaptureService, Inner};
use crate::capture::{
    model::{signal_matches_source, CaptureState, PipelineState, SegmentRecord},
    RuntimeSignal,
};
use crate::encoder::EncoderCommand;
use crate::retention::RetentionState;
use crossbeam_channel::Receiver;
use std::{sync::Weak, thread, time::Duration};
use tauri::Emitter;

pub(super) fn spawn(inner: Weak<Inner>, receiver: Receiver<RuntimeSignal>) {
    thread::spawn(move || loop {
        let Some(inner) = inner.upgrade() else { break };
        let service = CaptureService(inner);
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(RuntimeSignal::Healthy(source_id)) => service.mark_healthy(&source_id),
            Ok(RuntimeSignal::Paused(source_id)) => service.mark_paused(&source_id),
            Ok(RuntimeSignal::SourceUnavailable(source_id)) => service.mark_unavailable(&source_id),
            Ok(RuntimeSignal::CheckAvailability(source_id)) => {
                service.check_availability(&source_id)
            }
            Ok(RuntimeSignal::TransientFailure(source_id)) => {
                recovery::recover_transient(&service, &source_id)
            }
            Ok(RuntimeSignal::ClockDiscontinuity(source_id)) => {
                recovery::recover_from_clock_discontinuity(&service, &source_id)
            }
            Ok(RuntimeSignal::GeometryChanged(source_id, width, height)) => {
                resize::handle(&service, &source_id, width, height)
            }
            Ok(RuntimeSignal::EncoderStarted) => service.mark_encoder_started(),
            Ok(RuntimeSignal::EncoderStopped) => service.mark_encoder_stopped(),
            Ok(RuntimeSignal::EncoderFailed(code)) => service.mark_encoder_failed(code),
            Ok(RuntimeSignal::SegmentComplete(record)) => service.record_segment(record),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                service.refresh_dropped_frames();
                service.check_reappearance();
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    });
}

impl CaptureService {
    fn mark_encoder_started(&self) {
        if self.snapshot().pipeline != PipelineState::Encoding {
            self.update(|snapshot| {
                snapshot.pipeline = PipelineState::Encoding;
                snapshot.encoder_error_code = None;
            });
        }
    }

    fn mark_encoder_stopped(&self) {
        self.update(|snapshot| {
            snapshot.pipeline = PipelineState::Idle;
            snapshot.encoder_error_code = None;
        });
    }

    fn mark_encoder_failed(&self, code: String) {
        self.update(|snapshot| {
            snapshot.pipeline = PipelineState::Failed;
            snapshot.encoder_error_code = Some(code);
        });
    }

    pub(super) fn record_segment(&self, record: SegmentRecord) {
        let admitted = self.0.rolling.admit(&record);
        let retention = self.0.rolling.snapshot();
        let was_failed = self.snapshot().retention.state == RetentionState::Failed;
        if let Err(code) = &admitted {
            if !was_failed {
                self.log_retention(RetentionState::Failed, Some(code.clone()));
            }
        }
        self.update(|snapshot| {
            if admitted.is_ok() && snapshot.capture != CaptureState::Stopped {
                snapshot.pipeline = PipelineState::Encoding;
            } else if admitted.is_err() {
                snapshot.pipeline = PipelineState::Failed;
            }
            snapshot.encoder_error_code = None;
            snapshot.completed_segments = snapshot.completed_segments.saturating_add(1);
            snapshot.retention = retention;
        });
        if admitted.is_ok() {
            if let Some(app) = self.0.app.as_ref() {
                let _ = app.emit("segment-complete", record);
            }
        } else {
            let _ = self.0.encoder_controls.send(EncoderCommand::Halt);
        }
    }

    pub(super) fn is_current_source(&self, source_id: &str) -> bool {
        signal_matches_source(&self.snapshot(), source_id)
    }

    pub(super) fn mark_healthy(&self, source_id: &str) {
        // Only the monitor's automatic pause (stream still alive) may flip
        // back to `Capturing` here; after an explicit user pause there is no
        // stream, so a stale healthy signal would make the state lie.
        if self.is_user_paused() {
            return;
        }
        if self.is_current_source(source_id) && self.snapshot().capture == CaptureState::Paused {
            self.update(|snapshot| {
                snapshot.error_code = None;
                let _ = snapshot.set_capture(CaptureState::Capturing);
            });
            self.log_capture(CaptureState::Capturing, None);
        }
    }

    fn mark_paused(&self, source_id: &str) {
        if self.is_current_source(source_id) && self.snapshot().capture == CaptureState::Capturing {
            self.update(|snapshot| {
                snapshot.evidence_gap_count += 1;
                let _ = snapshot.set_capture(CaptureState::Paused);
            });
            self.log_capture(CaptureState::Paused, None);
        }
    }

    pub(super) fn mark_unavailable(&self, source_id: &str) {
        if !self.is_current_source(source_id)
            || self.snapshot().capture == CaptureState::SourceUnavailable
        {
            return;
        }
        self.update(|snapshot| {
            snapshot.error_code = Some("source_unavailable".into());
            snapshot.evidence_gap_count += 1;
            let _ = snapshot.set_capture(CaptureState::SourceUnavailable);
        });
        self.log_capture(
            CaptureState::SourceUnavailable,
            Some("source_unavailable".into()),
        );
    }

    fn check_availability(&self, source_id: &str) {
        if self.is_current_source(source_id) && !self.0.backend.source_exists(source_id) {
            self.mark_unavailable(source_id);
        }
    }

    fn check_reappearance(&self) {
        let snapshot = self.snapshot();
        if snapshot.capture != CaptureState::SourceUnavailable {
            return;
        }
        let Some(source) = snapshot.source else {
            return;
        };
        if self.0.backend.source_exists(&source.id) {
            let _ = self.switch_source(source);
        }
    }

    fn refresh_dropped_frames(&self) {
        let dropped = self.0.frames.dropped_count();
        if self.snapshot().dropped_frames != dropped {
            self.update(|_| {});
        }
    }
}
