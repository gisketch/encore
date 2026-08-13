use super::CaptureService;
use crate::capture::model::{CaptureState, PermissionState};
use crate::diagnostics::{label, DiagnosticDomain};
use crate::retention::RetentionState;

/// Narrow logging surface for the capture service: every method below just
/// renders a typed state (and stable code, when there is one) through the
/// shared `DiagnosticLog` handle already sitting on `Inner`. No logger is
/// threaded through call signatures — callers reach it via `self`.
impl CaptureService {
    pub(super) fn log_permission(&self, state: PermissionState, code: Option<String>) {
        self.0
            .diagnostics
            .record(DiagnosticDomain::Permission, label(&state), code);
    }

    pub(super) fn log_capture(&self, state: CaptureState, code: Option<String>) {
        self.0
            .diagnostics
            .record(DiagnosticDomain::Capture, label(&state), code);
    }

    pub(super) fn log_retention(&self, state: RetentionState, code: Option<String>) {
        self.0
            .diagnostics
            .record(DiagnosticDomain::Retention, label(&state), code);
    }
}
