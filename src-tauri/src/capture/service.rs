mod appearance;
mod control;
mod default_source;
mod diagnostics;
mod monitor;
mod pause;
mod persistence;
mod recovery;
mod resize;
mod switch;
#[cfg(test)]
mod tests;

use super::{
    backend::{CaptureBackend, RunningCapture, SystemCaptureBackend},
    mailbox::{latest_channel, LatestSender},
    model::{
        should_request_permission, CaptureSnapshot, CaptureSource, CaptureState, DiagnosticRecord,
        PermissionState, PipelineState,
    },
    permission::{PermissionProvider, SystemPermission},
    platform::NativeFrame,
    settings::{PersistedTarget, SettingsStore},
    RuntimeSignal,
};
use crate::diagnostics::DiagnosticLog;
use crate::encoder::{self, EncoderCommand};
use crate::retention::{RetentionState, RollingStore};
use crossbeam_channel::{bounded, Sender};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex, RwLock},
    thread,
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone)]
pub struct CaptureService(Arc<Inner>);

struct Inner {
    app: Option<AppHandle>,
    snapshot: RwLock<CaptureSnapshot>,
    operation: Mutex<()>,
    stream: Mutex<Option<Box<dyn RunningCapture>>>,
    permission: Box<dyn PermissionProvider>,
    backend: Box<dyn CaptureBackend>,
    frames: LatestSender<NativeFrame>,
    signals: Sender<RuntimeSignal>,
    encoder_controls: Sender<EncoderCommand>,
    rolling: RollingStore,
    settings: SettingsStore,
    // Doubles as the launch-time capture target and the Settings window's
    // "default source" display value: switching live source (bar) and
    // choosing a default (Settings) both write here, last write wins.
    default_target: RwLock<PersistedTarget>,
    appearance: RwLock<String>,
    diagnostics: DiagnosticLog,
    // Distinguishes an explicit user pause (stream torn down, must stay
    // paused) from the monitor's automatic blank/suspended pause, so queued
    // recovery or healthy signals cannot silently un-pause the user.
    user_paused: AtomicBool,
}

impl CaptureService {
    pub fn new(app: AppHandle, settings_path: PathBuf, diagnostics: DiagnosticLog) -> Self {
        let (frames, receiver) = latest_channel(3);
        let (signals, signal_receiver) = bounded(8);
        let (encoder_controls, encoder_control_receiver) = bounded(4);
        let permission: Box<dyn PermissionProvider> = Box::new(SystemPermission);
        let initial_permission = permission.current();
        let settings = SettingsStore::new(settings_path);
        let document = settings.load();
        let output_dir = app
            .path()
            .app_cache_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("encore"))
            .join("rolling-segments");
        let rolling = RollingStore::open(output_dir.clone(), document.retention_minutes)
            .unwrap_or_else(|code| {
                RollingStore::failed(output_dir.clone(), document.retention_minutes, &code)
            });
        let retention = rolling.snapshot();
        let pipeline = if retention.state == RetentionState::Failed {
            PipelineState::Failed
        } else {
            PipelineState::Idle
        };
        if retention.state == RetentionState::Failed {
            let _ = encoder_controls.send(EncoderCommand::Halt);
        }
        let service = Self(Arc::new(Inner {
            app: Some(app.clone()),
            snapshot: RwLock::new(CaptureSnapshot {
                permission: initial_permission,
                retention,
                pipeline,
                ..CaptureSnapshot::default()
            }),
            operation: Mutex::new(()),
            stream: Mutex::new(None),
            permission,
            backend: Box::new(SystemCaptureBackend),
            frames,
            signals: signals.clone(),
            encoder_controls,
            rolling,
            settings,
            default_target: RwLock::new(document.target),
            appearance: RwLock::new(document.appearance),
            diagnostics,
            user_paused: AtomicBool::new(false),
        }));
        let encoder_signals = signals.clone();
        thread::spawn(move || {
            encoder::run(
                receiver,
                encoder_control_receiver,
                output_dir,
                encoder_signals,
            );
        });
        monitor::spawn(Arc::downgrade(&service.0), signal_receiver);

        if initial_permission == PermissionState::Granted {
            let startup = service.clone();
            thread::spawn(move || {
                let _ = persistence::start_from_persisted(&startup);
            });
        }
        service
    }

    pub fn snapshot(&self) -> CaptureSnapshot {
        self.0
            .snapshot
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn rolling_store(&self) -> RollingStore {
        self.0.rolling.clone()
    }

    pub fn request_permission(&self) -> CaptureSnapshot {
        if !should_request_permission(self.snapshot().permission) {
            return self.snapshot();
        }
        let current = self.0.permission.current();
        let permission = if current == PermissionState::Granted {
            current
        } else {
            self.0.permission.request()
        };
        self.update(|snapshot| snapshot.permission = permission);
        self.log_permission(permission, None);
        if permission == PermissionState::Granted
            && self.snapshot().capture == CaptureState::Stopped
        {
            let startup = self.clone();
            thread::spawn(move || {
                let _ = persistence::start_from_persisted(&startup);
            });
        }
        self.snapshot()
    }

    pub fn diagnostics(&self) -> DiagnosticRecord {
        DiagnosticRecord::from(&self.snapshot())
    }

    pub fn set_retention_minutes(&self, minutes: u8) -> Result<CaptureSnapshot, String> {
        match self.0.rolling.set_minutes(minutes) {
            Ok(retention) => {
                self.update(|snapshot| snapshot.retention = retention);
                persistence::persist(self);
                Ok(self.snapshot())
            }
            Err(code) => {
                let retention = self.0.rolling.snapshot();
                let failed = retention.state == RetentionState::Failed;
                self.update(|snapshot| {
                    snapshot.retention = retention;
                    if failed {
                        snapshot.pipeline = PipelineState::Failed;
                    }
                });
                if failed {
                    let _ = self.0.encoder_controls.send(EncoderCommand::Halt);
                    self.log_retention(RetentionState::Failed, Some(code.clone()));
                }
                Err(code)
            }
        }
    }

    pub fn list_sources(&self) -> Result<Vec<CaptureSource>, String> {
        if self.0.permission.current() != PermissionState::Granted {
            return Err("capture_permission_required".into());
        }
        let _operation = self
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.0.backend.list_sources().map_err(str::to_owned)
    }

    pub fn start_default(&self) -> Result<CaptureSnapshot, String> {
        let _operation = self
            .0
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let source = self
            .0
            .backend
            .main_display_source()
            .map_err(|code| self.switch_failed(code, self.snapshot().capture))?;
        self.switch_source_locked(source)
    }

    pub fn switch_by_id(&self, source_id: &str) -> Result<CaptureSnapshot, String> {
        let source = self
            .list_sources()?
            .into_iter()
            .find(|source| source.id == source_id)
            .ok_or_else(|| "source_unavailable".to_string())?;
        let snapshot = self.switch_source(source)?;
        persistence::persist(self);
        Ok(snapshot)
    }

    pub fn retry(&self) -> Result<CaptureSnapshot, String> {
        match self.snapshot().source {
            Some(source) => self.switch_source(source),
            None => self.start_default(),
        }
    }

    fn update(&self, change: impl FnOnce(&mut CaptureSnapshot)) {
        let next = {
            let mut snapshot = self
                .0
                .snapshot
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            change(&mut snapshot);
            snapshot.dropped_frames = self.0.frames.dropped_count();
            snapshot.clone()
        };
        if let Some(app) = self.0.app.as_ref() {
            let _ = app.emit("capture-state-changed", next);
        }
    }
}
