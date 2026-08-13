mod backend;
mod evidence;
mod mailbox;
mod model;
mod permission;
mod platform;
mod service;
mod settings;
mod sources;

pub(crate) use evidence::{CaptureGeometry, EvidenceCaptureFacts};
pub use model::{CaptureSnapshot, CaptureSource, DiagnosticRecord, SegmentBoundary, SegmentRecord};
pub(crate) use model::{CaptureState, PipelineState, SourceKind};
pub(crate) use platform::NativeFrame;
pub use service::CaptureService;
pub(crate) use settings::HotkeyId;
pub use settings::{Hotkeys, SettingsSnapshot};

#[derive(Debug, Clone)]
pub(crate) enum RuntimeSignal {
    Healthy(String),
    Paused(String),
    SourceUnavailable(String),
    CheckAvailability(String),
    TransientFailure(String),
    ClockDiscontinuity(String),
    GeometryChanged(String, u32, u32),
    EncoderStarted,
    EncoderStopped,
    EncoderFailed(String),
    SegmentComplete(SegmentRecord),
}
