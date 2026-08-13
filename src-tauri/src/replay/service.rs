use super::{
    reveal::reveal_in_finder,
    state::{pending_snapshot, record_failure, snapshot, PendingReplay, ReplayServiceState},
    ReplaySnapshot, ShortcutRegistrationSnapshot,
};
use crate::{
    capture::{CaptureService, EvidenceCaptureFacts},
    diagnostics::{DiagnosticDomain, DiagnosticLog},
    packager::{current_sidecar_path, PackageRequest, ProcessFfmpegRunner, ReplayPackager},
    retention::{RollingStore, SegmentLease},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

type RevealSavedReplay = dyn Fn(&Path) -> Result<(), ()> + Send + Sync;

#[derive(Clone)]
pub(crate) struct ReplayService(Arc<ReplayInner>);

struct ReplayInner {
    rolling: RollingStore,
    clock: Arc<dyn ReplayClock>,
    packager: Arc<ReplayPackager>,
    capture_facts: Arc<dyn Fn() -> EvidenceCaptureFacts + Send + Sync>,
    reveal: Arc<RevealSavedReplay>,
    state: Mutex<ReplayServiceState>,
    diagnostics: DiagnosticLog,
}

struct PackageTask {
    replay_id: String,
    created_at_unix_ms: u64,
    lease: SegmentLease,
    capture: EvidenceCaptureFacts,
}

pub(crate) struct ReplayDispatch {
    pub(crate) snapshot: ReplaySnapshot,
    pub(crate) replay_id: Option<String>,
}

pub(crate) trait ReplayClock: Send + Sync {
    fn now(&self) -> Result<ReplayTime, ()>;
}

#[derive(Clone, Copy)]
pub(crate) struct ReplayTime {
    pub(crate) unix_ms: u64,
}

struct SystemReplayClock;

impl ReplayClock for SystemReplayClock {
    fn now(&self) -> Result<ReplayTime, ()> {
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ())
            .and_then(|duration| duration.as_millis().try_into().map_err(|_| ()))?;
        Ok(ReplayTime { unix_ms })
    }
}

impl ReplayService {
    pub(crate) fn new(
        rolling: RollingStore,
        capture: CaptureService,
        destination: PathBuf,
        diagnostics: DiagnosticLog,
    ) -> Result<Self, String> {
        let ffmpeg = current_sidecar_path("ffmpeg")?;
        let ffprobe = current_sidecar_path("ffprobe")?;
        let capture_facts = Arc::new(move || EvidenceCaptureFacts::from(&capture.snapshot()));
        Ok(Self::with_parts(
            rolling,
            Arc::new(SystemReplayClock),
            Arc::new(ReplayPackager::new(
                destination,
                Arc::new(ProcessFfmpegRunner::new(ffmpeg, ffprobe)),
            )),
            capture_facts,
            Arc::new(reveal_in_finder),
            diagnostics,
        ))
    }

    #[cfg(test)]
    pub(crate) fn trigger(&self) -> Result<ReplaySnapshot, String> {
        self.trigger_for_export().map(|dispatch| dispatch.snapshot)
    }

    pub(crate) fn trigger_for_export(&self) -> Result<ReplayDispatch, String> {
        let mut state = self.lock();
        if state.exporting_id.is_some() {
            return Ok(ReplayDispatch {
                snapshot: snapshot(&state),
                replay_id: None,
            });
        }
        let now = match self.0.clock.now() {
            Ok(now) => now,
            Err(()) => return Err(self.fail(&mut state, "replay_trigger_failed")),
        };
        let lease = match self.0.rolling.lease() {
            Ok(lease) => lease,
            Err(_) => return Err(self.fail(&mut state, "replay_retention_failed")),
        };
        let Some(replay) = pending_snapshot(&lease, state.next_id, now.unix_ms) else {
            return Err(self.fail(&mut state, "replay_unavailable"));
        };
        let Some(next_id) = state.next_id.checked_add(1) else {
            return Err(self.fail(&mut state, "replay_trigger_failed"));
        };

        state.next_id = next_id;
        let replay_id = replay.id.clone();
        let capture = (self.0.capture_facts)();
        let previous = state.pending.replace(PendingReplay {
            lease,
            snapshot: replay,
            capture,
        });
        state.exporting_id = Some(replay_id.clone());
        state.saved = None;
        state.error_code = None;
        drop(previous);
        Ok(ReplayDispatch {
            snapshot: snapshot(&state),
            replay_id: Some(replay_id),
        })
    }

    pub(crate) fn retry_for_export(&self) -> Result<ReplayDispatch, String> {
        let mut state = self.lock();
        if state.exporting_id.is_some() {
            return Ok(ReplayDispatch {
                snapshot: snapshot(&state),
                replay_id: None,
            });
        }
        let Some(replay_id) = state
            .pending
            .as_ref()
            .map(|pending| pending.snapshot.id.clone())
        else {
            return Err(self.fail(&mut state, "replay_unavailable"));
        };
        state.exporting_id = Some(replay_id.clone());
        state.error_code = None;
        Ok(ReplayDispatch {
            snapshot: snapshot(&state),
            replay_id: Some(replay_id),
        })
    }

    pub(crate) fn run_export(&self, replay_id: &str) -> ReplaySnapshot {
        let Some(task) = self.package_task(replay_id) else {
            return self.snapshot();
        };
        let result = self.0.packager.package(&PackageRequest {
            replay_id: &task.replay_id,
            created_at_unix_ms: task.created_at_unix_ms,
            lease: &task.lease,
            capture: task.capture.clone(),
        });
        drop(task);

        let mut state = self.lock();
        if state.exporting_id.as_deref() != Some(replay_id) {
            return snapshot(&state);
        }
        state.exporting_id = None;
        match result {
            Ok(saved) => {
                let pending = state.pending.take();
                if pending
                    .as_ref()
                    .is_none_or(|pending| pending.snapshot.id != replay_id)
                {
                    state.error_code = Some("replay_trigger_failed".into());
                    self.0.diagnostics.record(
                        DiagnosticDomain::Export,
                        "failed",
                        Some("replay_trigger_failed".into()),
                    );
                } else {
                    state.saved = Some(saved);
                    state.error_code = None;
                    self.0
                        .diagnostics
                        .record(DiagnosticDomain::Export, "saved", None);
                }
                drop(pending);
            }
            Err(code) => {
                self.0
                    .diagnostics
                    .record(DiagnosticDomain::Export, "failed", Some(code.clone()));
                state.error_code = Some(code);
            }
        }
        snapshot(&state)
    }

    pub(crate) fn reveal_saved(&self, replay_id: &str) -> Result<ReplaySnapshot, String> {
        let replay = {
            let state = self.lock();
            state
                .saved
                .as_ref()
                .filter(|saved| saved.id == replay_id)
                .cloned()
        };
        let Some(replay) = replay else {
            let mut state = self.lock();
            return Err(self.fail(&mut state, "export_reveal_failed"));
        };
        if (self.0.reveal)(&replay.replay_file).is_err() {
            let mut state = self.lock();
            return Err(self.fail(&mut state, "export_reveal_failed"));
        }
        let mut state = self.lock();
        state.error_code = None;
        Ok(snapshot(&state))
    }

    pub(crate) fn snapshot(&self) -> ReplaySnapshot {
        snapshot(&self.lock())
    }

    #[cfg(test)]
    pub(super) fn with_test_parts(
        rolling: RollingStore,
        clock: Arc<dyn ReplayClock>,
        packager: Arc<ReplayPackager>,
        reveal: Arc<RevealSavedReplay>,
    ) -> Self {
        Self::with_parts(
            rolling,
            clock,
            packager,
            Arc::new(EvidenceCaptureFacts::default),
            reveal,
            DiagnosticLog::disabled(),
        )
    }

    fn with_parts(
        rolling: RollingStore,
        clock: Arc<dyn ReplayClock>,
        packager: Arc<ReplayPackager>,
        capture_facts: Arc<dyn Fn() -> EvidenceCaptureFacts + Send + Sync>,
        reveal: Arc<RevealSavedReplay>,
        diagnostics: DiagnosticLog,
    ) -> Self {
        Self(Arc::new(ReplayInner {
            rolling,
            clock,
            packager,
            capture_facts,
            reveal,
            state: Mutex::new(ReplayServiceState {
                next_id: 1,
                pending: None,
                exporting_id: None,
                saved: None,
                error_code: None,
                shortcut: ShortcutRegistrationSnapshot::default(),
            }),
            diagnostics,
        }))
    }

    /// Records a typed export/replay failure and returns its stable code, so
    /// call sites can both fail the request and answer "why did the export
    /// fail" from the log in the same line.
    fn fail(&self, state: &mut ReplayServiceState, code: &str) -> String {
        let code = record_failure(state, code);
        self.0
            .diagnostics
            .record(DiagnosticDomain::Export, "failed", Some(code.clone()));
        code
    }

    pub(crate) fn set_shortcut_registration(&self, shortcut: ShortcutRegistrationSnapshot) {
        self.lock().shortcut = shortcut;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ReplayServiceState> {
        self.0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn package_task(&self, replay_id: &str) -> Option<PackageTask> {
        let state = self.lock();
        if state.exporting_id.as_deref() != Some(replay_id) {
            return None;
        }
        let pending = state.pending.as_ref()?;
        (pending.snapshot.id == replay_id).then(|| PackageTask {
            replay_id: replay_id.to_owned(),
            created_at_unix_ms: pending.snapshot.created_at_unix_ms,
            lease: pending.lease.clone(),
            capture: pending.capture.clone(),
        })
    }
}
