//! Two-pass ffmpeg palette pipeline for GIF export (grilled decision in
//! `docs/specs/2026-08-13-replay-editor.md`): pass 1 builds a palette
//! (`palettegen`), pass 2 applies it (`paletteuse`) while downscaling to
//! max 640px wide at 10fps. A dedicated trait — mirroring `ExportRunner`'s
//! shape rather than growing it — keeps GIF-only concerns out of the MP4
//! trim/concat path; `filter` is passed in by the caller (not baked into
//! the runner) so a fake can record and assert the exact filter string
//! each pass used.

use crate::packager::RunnerFailure;
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

/// The shared filter chain for both passes: downscale to max 640px wide
/// (height auto, even-rounded via `-2`) at 10fps, high-quality Lanczos
/// scaling. Applied identically in `palettegen` and `paletteuse` so the
/// palette is generated against the exact frames it will be applied to.
pub(crate) const GIF_FILTER: &str = "fps=10,scale=640:-2:flags=lanczos";

pub(crate) trait GifRunner: Send + Sync {
    /// Pass 1: analyzes `input` through `filter` and writes an optimal
    /// palette PNG to `palette`.
    fn palettegen(&self, input: &Path, filter: &str, palette: &Path) -> Result<(), RunnerFailure>;
    /// Pass 2: re-applies `filter` to `input` and dithers it against
    /// `palette`, writing the final GIF to `output`.
    fn paletteuse(
        &self,
        input: &Path,
        palette: &Path,
        filter: &str,
        output: &Path,
    ) -> Result<(), RunnerFailure>;
}

pub(crate) struct ProcessGifRunner {
    ffmpeg: PathBuf,
}

impl ProcessGifRunner {
    pub(crate) fn new(ffmpeg: PathBuf) -> Self {
        Self { ffmpeg }
    }
}

impl GifRunner for ProcessGifRunner {
    fn palettegen(&self, input: &Path, filter: &str, palette: &Path) -> Result<(), RunnerFailure> {
        let status = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(input)
            .args(["-vf", &format!("{filter},palettegen")])
            .arg(palette)
            .status()
            .map_err(command_failure)?;
        status.success().then_some(()).ok_or(RunnerFailure::Failed)
    }

    fn paletteuse(
        &self,
        input: &Path,
        palette: &Path,
        filter: &str,
        output: &Path,
    ) -> Result<(), RunnerFailure> {
        let status = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(input)
            .arg("-i")
            .arg(palette)
            .args(["-lavfi", &format!("{filter}[x];[x][1:v]paletteuse")])
            .arg(output)
            .status()
            .map_err(command_failure)?;
        status.success().then_some(()).ok_or(RunnerFailure::Failed)
    }
}

fn command_failure(error: io::Error) -> RunnerFailure {
    if error.kind() == io::ErrorKind::NotFound {
        RunnerFailure::Unavailable
    } else {
        RunnerFailure::Failed
    }
}
