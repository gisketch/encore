#[cfg(test)]
mod tests;

use crate::retention::RetentionSnapshot;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Unknown,
    PermissionRequired,
    PermissionDenied,
    RestartRequired,
    Granted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Stopped,
    Starting,
    Capturing,
    Paused,
    Recovering,
    SourceUnavailable,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Display,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Idle,
    Encoding,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentBoundary {
    Duration,
    SourceChanged,
    GeometryChanged,
    ClockDiscontinuity,
    CaptureEnded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentRecord {
    pub path: String,
    pub source_id: String,
    pub width: u32,
    pub height: u32,
    pub byte_size: u64,
    pub duration_micros: u64,
    pub session_start_micros: u64,
    pub session_end_micros: u64,
    pub completed_at_unix_ms: u64,
    pub boundary: SegmentBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    pub id: String,
    pub kind: SourceKind,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub is_main: bool,
    /// Best-effort window identity for settings persistence. `None` for
    /// display sources.
    pub bundle_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshot {
    pub permission: PermissionState,
    pub capture: CaptureState,
    pub source: Option<CaptureSource>,
    pub dropped_frames: u64,
    pub retry_count: u8,
    pub error_code: Option<String>,
    pub evidence_gap_count: u64,
    pub pipeline: PipelineState,
    pub completed_segments: u64,
    pub encoder_error_code: Option<String>,
    pub retention: RetentionSnapshot,
    /// Stable code shown when a persisted target could not be resolved at
    /// launch and capture fell back to the default display.
    pub source_notice: Option<String>,
}

impl Default for CaptureSnapshot {
    fn default() -> Self {
        Self {
            permission: PermissionState::Unknown,
            capture: CaptureState::Stopped,
            source: None,
            dropped_frames: 0,
            retry_count: 0,
            error_code: None,
            evidence_gap_count: 0,
            pipeline: PipelineState::Idle,
            completed_segments: 0,
            encoder_error_code: None,
            retention: RetentionSnapshot::default(),
            source_notice: None,
        }
    }
}

impl CaptureSnapshot {
    pub fn set_capture(&mut self, next: CaptureState) -> Result<(), &'static str> {
        let valid = matches!(
            (self.capture, next),
            (CaptureState::Stopped, CaptureState::Starting)
                | (CaptureState::Starting, CaptureState::Capturing)
                | (CaptureState::Starting, CaptureState::Paused)
                | (CaptureState::Starting, CaptureState::Failed)
                | (CaptureState::Starting, CaptureState::SourceUnavailable)
                | (CaptureState::Stopped, CaptureState::Failed)
                | (CaptureState::Stopped, CaptureState::SourceUnavailable)
                | (CaptureState::Capturing, CaptureState::Paused)
                | (CaptureState::Capturing, CaptureState::Starting)
                | (CaptureState::Capturing, CaptureState::SourceUnavailable)
                | (CaptureState::Capturing, CaptureState::Failed)
                | (CaptureState::Paused, CaptureState::Capturing)
                | (CaptureState::Paused, CaptureState::Starting)
                | (CaptureState::Paused, CaptureState::SourceUnavailable)
                | (CaptureState::Paused, CaptureState::Failed)
                | (CaptureState::Capturing, CaptureState::Recovering)
                | (CaptureState::Paused, CaptureState::Recovering)
                | (CaptureState::Recovering, CaptureState::Starting)
                | (CaptureState::Recovering, CaptureState::Failed)
                | (CaptureState::Recovering, CaptureState::SourceUnavailable)
                | (CaptureState::Failed, CaptureState::Recovering)
                | (CaptureState::SourceUnavailable, CaptureState::Starting)
                | (CaptureState::SourceUnavailable, CaptureState::Capturing)
                | (CaptureState::SourceUnavailable, CaptureState::Failed)
                | (CaptureState::Failed, CaptureState::Starting)
                | (CaptureState::Failed, CaptureState::SourceUnavailable)
                | (_, CaptureState::Stopped)
        ) || self.capture == next;

        if !valid {
            return Err("invalid_capture_transition");
        }
        self.capture = next;
        Ok(())
    }
}

pub const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

pub fn should_request_permission(state: PermissionState) -> bool {
    state == PermissionState::PermissionRequired
}

pub fn signal_matches_source(snapshot: &CaptureSnapshot, source_id: &str) -> bool {
    snapshot.source.as_ref().map(|source| source.id.as_str()) == Some(source_id)
}

pub fn switch_failure_state(
    previous: CaptureState,
    had_active_stream: bool,
    previous_source_available: bool,
) -> CaptureState {
    match previous {
        CaptureState::Capturing | CaptureState::Paused
            if had_active_stream && previous_source_available =>
        {
            previous
        }
        CaptureState::Capturing | CaptureState::Paused if !previous_source_available => {
            CaptureState::SourceUnavailable
        }
        CaptureState::SourceUnavailable => CaptureState::SourceUnavailable,
        _ => CaptureState::Failed,
    }
}

pub fn aspect_fit(source_width: u32, source_height: u32) -> (u32, u32) {
    if source_width == 0 || source_height == 0 {
        return (1, 1);
    }
    let scale = f64::min(
        1.0,
        f64::min(1920.0 / source_width as f64, 1080.0 / source_height as f64),
    );
    let width = ((source_width as f64 * scale).round() as u32).max(1);
    let height = ((source_height as f64 * scale).round() as u32).max(1);
    (width, height)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRecord {
    pub source_id: Option<String>,
    pub source_kind: Option<SourceKind>,
    pub geometry: Option<(u32, u32)>,
    pub capture: CaptureState,
    pub error_code: Option<String>,
    pub retry_count: u8,
    pub dropped_frames: u64,
}

impl From<&CaptureSnapshot> for DiagnosticRecord {
    fn from(value: &CaptureSnapshot) -> Self {
        Self {
            source_id: value.source.as_ref().map(|source| source.id.clone()),
            source_kind: value.source.as_ref().map(|source| source.kind),
            geometry: value
                .source
                .as_ref()
                .map(|source| (source.width, source.height)),
            capture: value.capture,
            error_code: value.error_code.clone(),
            retry_count: value.retry_count,
            dropped_frames: value.dropped_frames,
        }
    }
}
