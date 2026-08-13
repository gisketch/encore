use super::model::{CaptureSnapshot, CaptureState, PipelineState, SourceKind};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceCaptureFacts {
    pub source_kind: Option<SourceKind>,
    pub geometry: Option<CaptureGeometry>,
    pub capture_state: CaptureState,
    pub encoder_state: PipelineState,
    pub retention_minutes: u8,
    pub dropped_frames: u64,
    pub evidence_gap_count: u64,
    pub retry_count: u8,
}

impl Default for EvidenceCaptureFacts {
    fn default() -> Self {
        Self {
            source_kind: None,
            geometry: None,
            capture_state: CaptureState::Stopped,
            encoder_state: PipelineState::Idle,
            retention_minutes: 10,
            dropped_frames: 0,
            evidence_gap_count: 0,
            retry_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureGeometry {
    pub width: u32,
    pub height: u32,
}

impl From<&CaptureSnapshot> for EvidenceCaptureFacts {
    fn from(value: &CaptureSnapshot) -> Self {
        Self {
            source_kind: value.source.as_ref().map(|source| source.kind),
            geometry: value.source.as_ref().map(|source| CaptureGeometry {
                width: source.width,
                height: source.height,
            }),
            capture_state: value.capture,
            encoder_state: value.pipeline,
            retention_minutes: value.retention.minutes,
            dropped_frames: value.dropped_frames,
            evidence_gap_count: value.evidence_gap_count,
            retry_count: value.retry_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{CaptureSource, SourceKind};

    #[test]
    fn evidence_facts_exclude_source_identity_and_label() {
        let snapshot = CaptureSnapshot {
            source: Some(CaptureSource {
                id: "window:42".into(),
                kind: SourceKind::Window,
                label: "Private document title".into(),
                width: 900,
                height: 700,
                is_main: false,
                bundle_id: Some("com.example.app".into()),
                title: Some("Private document title".into()),
            }),
            ..CaptureSnapshot::default()
        };
        let json = serde_json::to_string(&EvidenceCaptureFacts::from(&snapshot)).unwrap();
        assert!(!json.contains("window:42"));
        assert!(!json.contains("Private document title"));
        assert!(json.contains("900"));
        assert!(json.contains("700"));
    }
}
