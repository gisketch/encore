use crate::{
    capture::{CaptureGeometry, CaptureState, EvidenceCaptureFacts, PipelineState, SourceKind},
    retention::SegmentLease,
};
use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

pub(crate) const REPLAY_FILENAME: &str = "replay.mp4";
pub(crate) const METADATA_FILENAME: &str = "metadata.json";

pub(crate) struct PackageRequest<'lease> {
    pub replay_id: &'lease str,
    pub created_at_unix_ms: u64,
    pub lease: &'lease SegmentLease,
    pub capture: EvidenceCaptureFacts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BundleStreamDescription {
    pub kind: String,
    pub codec: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub pixel_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BundleMediaDescription {
    pub streams: Vec<BundleStreamDescription>,
}

impl BundleMediaDescription {
    pub(crate) fn is_supported_h264_video(&self) -> bool {
        let mut videos = self.streams.iter().filter(|stream| stream.kind == "video");
        let Some(video) = videos.next() else {
            return false;
        };
        videos.next().is_none()
            && video.codec == "h264"
            && video.width.is_some_and(|width| width > 0)
            && video.height.is_some_and(|height| height > 0)
            && video
                .pixel_format
                .as_deref()
                .is_some_and(|format| !format.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SavedReplay {
    pub id: String,
    pub display_name: String,
    pub bundle_directory: PathBuf,
    pub replay_file: PathBuf,
    pub metadata_file: PathBuf,
}

pub(crate) fn display_name(created_at_unix_ms: u64, suffix: u32) -> Result<String, String> {
    let created_at = i64::try_from(created_at_unix_ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .ok_or_else(|| "export_destination_unavailable".to_string())?
        .with_timezone(&Local);
    let suffix = if suffix == 0 {
        String::new()
    } else {
        format!(" {suffix}")
    };
    Ok(format!(
        "Encore Replay {}{suffix}",
        created_at.format("%Y-%m-%d %H.%M.%S")
    ))
}

pub(crate) fn metadata_json(request: &PackageRequest<'_>) -> Result<Vec<u8>, String> {
    let segments = request.lease.segments();
    let first = segments
        .first()
        .ok_or_else(|| "export_incompatible_segments".to_string())?;
    let evidence_start_unix_ms = segments
        .iter()
        .map(|segment| segment.evidence_start_unix_ms())
        .min()
        .unwrap_or_else(|| first.evidence_start_unix_ms());
    let evidence_end_unix_ms = segments
        .iter()
        .map(|segment| segment.evidence_end_unix_ms())
        .max()
        .unwrap_or_else(|| first.evidence_end_unix_ms());
    let input_bytes = segments.iter().fold(0_u64, |total, segment| {
        total.saturating_add(segment.byte_size)
    });
    let metadata = BundleMetadata {
        schema_version: 1,
        replay_id: request.replay_id,
        created_at_unix_ms: request.created_at_unix_ms,
        evidence: EvidenceMetadata {
            start_unix_ms: evidence_start_unix_ms,
            end_unix_ms: evidence_end_unix_ms,
            segment_count: segments.len().try_into().unwrap_or(u64::MAX),
            input_bytes,
        },
        app: AppMetadata {
            name: "Encore",
            version: env!("CARGO_PKG_VERSION"),
        },
        environment: EnvironmentMetadata {
            os_name: std::env::consts::OS,
            os_version: os_version(),
            architecture: std::env::consts::ARCH,
        },
        capture: CaptureMetadata {
            source_kind: request.capture.source_kind,
            geometry: request.capture.geometry,
            state: request.capture.capture_state,
            encoder: EncoderMetadata {
                codec: "h264",
                keyframe_interval_seconds: 1,
                state: request.capture.encoder_state,
            },
            retention_minutes: request.capture.retention_minutes,
        },
        counters: CaptureCounters {
            dropped_frames: request.capture.dropped_frames,
            evidence_gap_count: request.capture.evidence_gap_count,
            retry_count: request.capture.retry_count,
        },
        output: OutputMetadata {
            video_filename: REPLAY_FILENAME,
        },
    };
    serde_json::to_vec_pretty(&metadata).map_err(|_| "export_metadata_failed".to_string())
}

fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        "unknown".into()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleMetadata<'a> {
    schema_version: u8,
    replay_id: &'a str,
    created_at_unix_ms: u64,
    evidence: EvidenceMetadata,
    app: AppMetadata<'a>,
    environment: EnvironmentMetadata,
    capture: CaptureMetadata,
    counters: CaptureCounters,
    output: OutputMetadata,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceMetadata {
    start_unix_ms: u64,
    end_unix_ms: u64,
    segment_count: u64,
    input_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata<'a> {
    name: &'a str,
    version: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentMetadata {
    os_name: &'static str,
    os_version: String,
    architecture: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureMetadata {
    source_kind: Option<SourceKind>,
    geometry: Option<CaptureGeometry>,
    state: CaptureState,
    encoder: EncoderMetadata,
    retention_minutes: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EncoderMetadata {
    codec: &'static str,
    keyframe_interval_seconds: u8,
    state: PipelineState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureCounters {
    dropped_frames: u64,
    evidence_gap_count: u64,
    retry_count: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputMetadata {
    video_filename: &'static str,
}
