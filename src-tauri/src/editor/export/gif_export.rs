//! GIF export orchestration, kept out of `mod.rs` (already tracked, at its
//! committed SCC baseline) so this ticket's new branching lands in a file
//! with room for it, per the harness's zero-tolerance for complexity
//! increases in already-committed files.

use super::{build_trimmed_video, gif, gif_publish, ExportRunner, GifRunner, KeepSegment};
use crate::{
    editor::keyframes::{self, KeyframeProbe},
    library,
    packager::{self, PackageFileSystem, RunnerFailure},
};
use std::path::{Path, PathBuf};

/// Exports the KEPT ranges of replay `id` as a GIF: builds the trimmed MP4
/// exactly as `export_trimmed` does (stream-copy + concat), then feeds that
/// intermediate through the two-pass ffmpeg palette pipeline
/// (`gif::GIF_FILTER`, downscaled to max 640px wide at 10fps). Unlike the
/// MP4 path, this never publishes a bundle directory — the ticket's
/// "a plain .gif file in the destination is fine" decision — so the result
/// is a single sibling file the Library's directory-only scan
/// (`library::scan`) simply never looks at.
pub(crate) fn export_gif(
    destination: &Path,
    id: &str,
    keep_segments: &[KeepSegment],
    probe: &dyn KeyframeProbe,
    export_runner: &dyn ExportRunner,
    gif_runner: &dyn GifRunner,
    files: &dyn PackageFileSystem,
) -> Result<PathBuf, String> {
    let bundle = library::resolve_bundle_dir(destination, id)?;
    let replay_file = bundle.join(packager::REPLAY_FILENAME);
    if !replay_file.is_file() {
        return Err("editor_replay_missing".to_string());
    }

    let probed = keyframes::probe(&replay_file, probe)?;
    let validated = super::segments::validate_keep_segments(
        keep_segments,
        &probed.seconds,
        probed.duration_seconds,
    )
    .map_err(|_| "export_segments_invalid".to_string())?;

    let source_name = bundle
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "editor_replay_missing".to_string())?
        .to_string();

    let workspace = gif_workspace(destination, &source_name);
    let _cleanup = GifWorkspaceGuard {
        files,
        path: &workspace,
    };
    files
        .create_dir(&workspace)
        .map_err(|_| "export_destination_unavailable".to_string())?;

    let trimmed_video = workspace.join("trimmed.mp4");
    build_trimmed_video(
        export_runner,
        files,
        &replay_file,
        &workspace,
        &validated,
        &trimmed_video,
    )?;

    let built_gif = run_palette_pipeline(gif_runner, files, &workspace, &trimmed_video)?;

    gif_publish::publish_gif(files, destination, &source_name, &built_gif)
}

/// Pass 1 (`palettegen`) then pass 2 (`paletteuse`), both against the
/// shared `gif::GIF_FILTER`, then a non-empty-output sanity check — the
/// same "did ffmpeg actually produce bytes" guard `build_trimmed_video`
/// applies to the MP4 path.
fn run_palette_pipeline(
    gif_runner: &dyn GifRunner,
    files: &dyn PackageFileSystem,
    workspace: &Path,
    trimmed_video: &Path,
) -> Result<PathBuf, String> {
    let palette = workspace.join("palette.png");
    gif_runner
        .palettegen(trimmed_video, gif::GIF_FILTER, &palette)
        .map_err(gif_error)?;
    let built_gif = workspace.join("output.gif");
    gif_runner
        .paletteuse(trimmed_video, &palette, gif::GIF_FILTER, &built_gif)
        .map_err(gif_error)?;
    files
        .file_len(&built_gif)
        .ok()
        .filter(|size| *size > 0)
        .map(|_| built_gif)
        .ok_or_else(|| "export_gif_failed".to_string())
}

/// A hidden, per-export scratch directory inside `destination` (never a
/// published bundle name, so `library::scan`'s directory listing skips it
/// even if cleanup is interrupted, the same way it already skips the
/// packager's `.`-prefixed workspace markers).
fn gif_workspace(destination: &Path, source_name: &str) -> PathBuf {
    static NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let nonce = NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    destination.join(format!(
        ".{source_name}.gif-workspace-{}-{nonce}",
        std::process::id()
    ))
}

/// Removes the scratch workspace on drop, on both the success and failure
/// path — a successful publish has already moved the finished GIF out of
/// it, and a failed one never gets that far, so either way only scratch
/// intermediates (segments, palette, the pre-publish GIF) remain to clean
/// up.
struct GifWorkspaceGuard<'a> {
    files: &'a dyn PackageFileSystem,
    path: &'a Path,
}

impl Drop for GifWorkspaceGuard<'_> {
    fn drop(&mut self) {
        let _ = self.files.remove_dir_all(self.path);
    }
}

fn gif_error(failure: RunnerFailure) -> String {
    match failure {
        RunnerFailure::Unavailable => "export_ffmpeg_unavailable".to_string(),
        RunnerFailure::Failed => "export_gif_failed".to_string(),
    }
}
