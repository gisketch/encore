use super::CaptureService;
use crate::capture::model::{CaptureSnapshot, CaptureState, PipelineState};
use crate::encoder::EncoderCommand;

impl CaptureService {
    pub fn stop(&self) -> CaptureSnapshot {
        let _operation = self
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.update(|snapshot| {
            let _ = snapshot.set_capture(CaptureState::Stopped);
            snapshot.source = None;
            snapshot.retry_count = 0;
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
        self.log_capture(CaptureState::Stopped, None);
        self.snapshot()
    }
}
