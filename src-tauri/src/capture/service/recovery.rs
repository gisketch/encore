use super::CaptureService;
use crate::capture::model::{CaptureState, PermissionState, RETRY_DELAYS};
use std::thread;

/// Stream-error trigger: ScreenCaptureKit reported `did_stop_with_error` for
/// the current source (this is also how a real sleep most often surfaces,
/// since sleep reaches ScreenCaptureKit indirectly rather than as its own
/// event).
pub(super) fn recover_transient(service: &CaptureService, source_id: &str) {
    recover(service, source_id, "transient_stream_failure");
}

/// Clock-discontinuity trigger: the encoder observed a multi-second jump in
/// frame timestamps for the current source with no corresponding stream
/// error — the signature of a sleep/wake gap that ScreenCaptureKit didn't
/// report as a hard failure. Recovery keys off this in addition to stream
/// errors so wake is not detected solely through power notifications.
pub(super) fn recover_from_clock_discontinuity(service: &CaptureService, source_id: &str) {
    recover(service, source_id, "clock_discontinuity");
}

/// Single bounded-retry recovery path shared by every trigger above (and, by
/// extension, wake, since wake surfaces here as one of those triggers): sets
/// `Recovering`, then restarts through `switch_source_locked` — the same
/// restart mechanism `resume` uses — with a shared bounded delay schedule.
/// Exhausting the schedule lands in `Failed` with a stable `retry_exhausted`
/// code and the rail's one-click retry.
fn recover(service: &CaptureService, source_id: &str, reason: &'static str) {
    // An explicit user pause outranks recovery: a signal queued before (or
    // raised by) the pause's own stream teardown must not restart capture.
    if service.is_user_paused() || !service.is_current_source(source_id) {
        return;
    }
    if service.0.permission.current() != PermissionState::Granted {
        service.update(|snapshot| {
            snapshot.permission = PermissionState::PermissionDenied;
            snapshot.error_code = Some("capture_permission_revoked".into());
            let _ = snapshot.set_capture(CaptureState::Failed);
        });
        service.log_permission(
            PermissionState::PermissionDenied,
            Some("capture_permission_revoked".into()),
        );
        service.log_capture(
            CaptureState::Failed,
            Some("capture_permission_revoked".into()),
        );
        return;
    }
    let Some(source) = service.snapshot().source else {
        return;
    };
    if !service.0.backend.source_exists(&source.id) {
        service.mark_unavailable(&source.id);
        return;
    }

    {
        let _operation = service
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !service.is_current_source(source_id) {
            return;
        }
        let stopped_stream = service
            .0
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        drop(stopped_stream);
    }

    for (index, delay) in RETRY_DELAYS.iter().enumerate() {
        if !service.is_current_source(source_id) {
            return;
        }
        service.update(|snapshot| {
            snapshot.retry_count = (index + 1) as u8;
            snapshot.error_code = Some(reason.into());
            let _ = snapshot.set_capture(CaptureState::Recovering);
        });
        service.log_capture(CaptureState::Recovering, Some(reason.into()));
        thread::sleep(*delay);
        if !service.is_current_source(source_id) {
            return;
        }
        let _operation = service
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !service.is_current_source(source_id) {
            return;
        }
        if service.switch_source_locked(source.clone()).is_ok() {
            return;
        }
    }
    service.update(|snapshot| {
        snapshot.error_code = Some("retry_exhausted".into());
        let _ = snapshot.set_capture(CaptureState::Failed);
    });
    service.log_capture(CaptureState::Failed, Some("retry_exhausted".into()));
}
