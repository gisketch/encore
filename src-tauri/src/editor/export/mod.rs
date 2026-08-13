pub(crate) mod commands;
mod gif;
mod gif_export;
mod gif_publish;
mod publish;
mod runner;
mod segments;
#[cfg(test)]
mod tests;

use crate::{
    editor::keyframes::{self, KeyframeProbe},
    library,
    packager::{self, PackageFileSystem, RunnerFailure},
};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[allow(unused_imports)]
pub(crate) use gif::{GifRunner, ProcessGifRunner, GIF_FILTER};
#[allow(unused_imports)]
pub(crate) use gif_export::export_gif;
#[allow(unused_imports)]
pub(crate) use runner::{ExportRunner, ProcessExportRunner};
#[allow(unused_imports)]
pub(crate) use segments::KeepSegment;

pub(crate) struct ExportedReplay {
    pub bundle_directory: PathBuf,
}

/// Exports the KEPT ranges (the complement of the editor's trims/cuts) of
/// replay `id` as a lossless, stream-copied sibling bundle. Every
/// endpoint the frontend sends has already been snapped to a keyframe
/// there, but this never trusts that: it re-probes the real keyframe list
/// straight off the source file and rejects (rather than silently
/// re-encoding) anything that does not land on a real snap point. The
/// source bundle is only ever read, never written.
pub(crate) fn export_trimmed(
    destination: &Path,
    id: &str,
    keep_segments: &[KeepSegment],
    probe: &dyn KeyframeProbe,
    runner: &dyn ExportRunner,
    files: &dyn PackageFileSystem,
) -> Result<ExportedReplay, String> {
    let bundle = library::resolve_bundle_dir(destination, id)?;
    let replay_file = bundle.join(packager::REPLAY_FILENAME);
    if !replay_file.is_file() {
        return Err("editor_replay_missing".to_string());
    }

    let probed = keyframes::probe(&replay_file, probe)?;
    let validated =
        segments::validate_keep_segments(keep_segments, &probed.seconds, probed.duration_seconds)
            .map_err(|_| "export_segments_invalid".to_string())?;

    let source_name = bundle
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "editor_replay_missing".to_string())?
        .to_string();
    let source_metadata = read_source_metadata(&bundle);

    let published = publish::publish_trimmed(files, destination, &source_name, |partial| {
        write_trimmed_bundle(
            runner,
            files,
            &replay_file,
            partial,
            &validated,
            id,
            source_metadata.as_ref(),
        )
    })?;

    Ok(ExportedReplay {
        bundle_directory: published,
    })
}

fn write_trimmed_bundle(
    runner: &dyn ExportRunner,
    files: &dyn PackageFileSystem,
    source_replay: &Path,
    partial: &Path,
    segments: &[KeepSegment],
    source_id: &str,
    source_metadata: Option<&serde_json::Value>,
) -> Result<(), String> {
    let replay_output = partial.join(packager::REPLAY_FILENAME);
    build_trimmed_video(
        runner,
        files,
        source_replay,
        partial,
        segments,
        &replay_output,
    )?;

    let metadata = trimmed_metadata_json(source_metadata, source_id, segments)?;
    files
        .write_synced(&partial.join(packager::METADATA_FILENAME), &metadata)
        .map_err(|_| "export_metadata_failed".to_string())?;
    files
        .sync_dir(partial)
        .map_err(|_| "export_destination_unavailable".to_string())
}

/// Trims each kept range from `source_replay` into `workspace`, concatting
/// them (when there is more than one) into a single file at `output`.
/// Shared by the MP4 bundle path (`write_trimmed_bundle`, `output` lands
/// inside the published bundle) and the GIF path (`export_gif`, `output` is
/// a scratch intermediate that never leaves the workspace) so both formats
/// cut the exact same way.
fn build_trimmed_video(
    runner: &dyn ExportRunner,
    files: &dyn PackageFileSystem,
    source_replay: &Path,
    workspace: &Path,
    segments: &[KeepSegment],
    output: &Path,
) -> Result<(), String> {
    let mut segment_paths = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let segment_output = workspace.join(format!("segment-{index}.mp4"));
        runner
            .trim(source_replay, segment.start, segment.end, &segment_output)
            .map_err(trim_error)?;
        segment_paths.push(segment_output);
    }

    if segment_paths.len() == 1 {
        files
            .rename(&segment_paths[0], output)
            .map_err(|_| "export_destination_unavailable".to_string())?;
    } else {
        let manifest = workspace.join("concat.txt");
        files
            .write_synced(&manifest, &concat_manifest(&segment_paths))
            .map_err(|_| "export_destination_unavailable".to_string())?;
        runner.concat(&manifest, output).map_err(concat_error)?;
        files
            .remove_file(&manifest)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        for path in &segment_paths {
            files
                .remove_file(path)
                .map_err(|_| "export_destination_unavailable".to_string())?;
        }
    }

    if files
        .file_len(output)
        .ok()
        .filter(|size| *size > 0)
        .is_none()
    {
        return Err("export_concat_failed".to_string());
    }
    files
        .sync_file(output)
        .map_err(|_| "export_destination_unavailable".to_string())
}

fn trim_error(failure: RunnerFailure) -> String {
    match failure {
        RunnerFailure::Unavailable => "export_ffmpeg_unavailable".to_string(),
        RunnerFailure::Failed => "export_trim_failed".to_string(),
    }
}

fn concat_error(failure: RunnerFailure) -> String {
    match failure {
        RunnerFailure::Unavailable => "export_ffmpeg_unavailable".to_string(),
        RunnerFailure::Failed => "export_concat_failed".to_string(),
    }
}

/// Same manifest format/escaping as `packager::package`'s concat
/// manifest (ffmpeg's concat-demuxer `file '<path>'` lines).
fn concat_manifest(paths: &[PathBuf]) -> Vec<u8> {
    paths
        .iter()
        .map(|path| {
            let text = path.to_string_lossy();
            format!(
                "file '{}'\n",
                text.replace('\\', "\\\\").replace('\'', "'\\\\''")
            )
        })
        .collect::<String>()
        .into_bytes()
}

fn read_source_metadata(bundle: &Path) -> Option<serde_json::Value> {
    let bytes = std::fs::read(bundle.join(packager::METADATA_FILENAME)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Source metadata plus `trimmed: true`, `sourceReplayId`, and the
/// applied `keptSegments` list — the provenance trail the spec's grilled
/// decisions require so an exported clip stays auditable back to its
/// source evidence bundle.
fn trimmed_metadata_json(
    source: Option<&serde_json::Value>,
    source_id: &str,
    segments: &[KeepSegment],
) -> Result<Vec<u8>, String> {
    let mut value = source.cloned().unwrap_or_else(|| serde_json::json!({}));
    value["createdAtUnixMs"] = serde_json::json!(created_at_unix_ms());
    value["trimmed"] = serde_json::json!(true);
    value["sourceReplayId"] = serde_json::json!(source_id);
    value["keptSegments"] =
        serde_json::to_value(segments).map_err(|_| "export_metadata_failed".to_string())?;
    serde_json::to_vec_pretty(&value).map_err(|_| "export_metadata_failed".to_string())
}

fn created_at_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since_epoch| u64::try_from(since_epoch.as_millis()).ok())
        .unwrap_or(0)
}
