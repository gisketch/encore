//! Tauri command wrapper for lossless export, kept in its own file (rather
//! than growing `editor::commands`, already tracked and near the
//! harness's ceiling) so this ticket adds no complexity to an existing
//! file.

use super::{export_gif, export_trimmed, KeepSegment, ProcessExportRunner, ProcessGifRunner};
use crate::{capture::CaptureService, editor::keyframes::ProcessKeyframeProbe, packager};

/// Exports the KEPT ranges of replay `id` as a lossless, stream-copied
/// sibling `(trimmed)` bundle in the current save destination. Returns
/// the new bundle's id (its folder name) so the frontend can drive the
/// "Show in Library" affordance.
#[tauri::command]
pub(crate) fn export_trimmed_replay(
    service: tauri::State<'_, CaptureService>,
    id: String,
    keep_segments: Vec<KeepSegment>,
) -> Result<String, String> {
    let destination = service.resolved_save_destination();
    let ffprobe = packager::current_sidecar_path("ffprobe")
        .map_err(|_| "editor_ffprobe_unavailable".to_string())?;
    let ffmpeg = packager::current_sidecar_path("ffmpeg")
        .map_err(|_| "export_ffmpeg_unavailable".to_string())?;
    let probe = ProcessKeyframeProbe::new(ffprobe);
    let runner = ProcessExportRunner::new(ffmpeg);
    let files = packager::SystemPackageFileSystem;

    let exported = export_trimmed(&destination, &id, &keep_segments, &probe, &runner, &files)?;

    exported
        .bundle_directory
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| "export_destination_unavailable".to_string())
}

/// Exports the KEPT ranges of replay `id` as a GIF (two-pass ffmpeg palette
/// pipeline, downscaled to max 640px wide at 10fps) in the current save
/// destination. Returns the published GIF's absolute path, which the
/// frontend needs in full (unlike the MP4 bundle id) to hand straight to
/// `copy_export_to_clipboard`.
#[tauri::command]
pub(crate) fn export_gif_replay(
    service: tauri::State<'_, CaptureService>,
    id: String,
    keep_segments: Vec<KeepSegment>,
) -> Result<String, String> {
    let destination = service.resolved_save_destination();
    let ffprobe = packager::current_sidecar_path("ffprobe")
        .map_err(|_| "editor_ffprobe_unavailable".to_string())?;
    let ffmpeg = packager::current_sidecar_path("ffmpeg")
        .map_err(|_| "export_ffmpeg_unavailable".to_string())?;
    let probe = ProcessKeyframeProbe::new(ffprobe);
    let export_runner = ProcessExportRunner::new(ffmpeg.clone());
    let gif_runner = ProcessGifRunner::new(ffmpeg);
    let files = packager::SystemPackageFileSystem;

    let published = export_gif(
        &destination,
        &id,
        &keep_segments,
        &probe,
        &export_runner,
        &gif_runner,
        &files,
    )?;

    Ok(published.to_string_lossy().into_owned())
}
