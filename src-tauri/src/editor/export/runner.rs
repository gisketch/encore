use crate::packager::RunnerFailure;
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

/// Ffmpeg operations the export pipeline needs beyond
/// `packager::FfmpegRunner` (a stream-copy trim of a single range). A
/// dedicated trait — rather than growing the packager's own trait/tests —
/// keeps this ticket's new files self-contained, per the harness's rule
/// against increasing complexity in already-tracked files; `concat` is
/// deliberately the same minimal shape `packager::ProcessFfmpegRunner::
/// concat` uses so exported output honors the same stream-copy contract.
pub(crate) trait ExportRunner: Send + Sync {
    fn trim(&self, input: &Path, start: f64, end: f64, output: &Path) -> Result<(), RunnerFailure>;
    fn concat(&self, manifest: &Path, output: &Path) -> Result<(), RunnerFailure>;
}

pub(crate) struct ProcessExportRunner {
    ffmpeg: PathBuf,
}

impl ProcessExportRunner {
    pub(crate) fn new(ffmpeg: PathBuf) -> Self {
        Self { ffmpeg }
    }
}

impl ExportRunner for ProcessExportRunner {
    /// `-ss <start> -to <end> -i <input> -c copy <output>` — input seeking
    /// with a stream copy, so ffmpeg never decodes/re-encodes a frame; the
    /// caller has already re-validated both endpoints land on real
    /// keyframes (or a clip boundary) before this ever runs.
    fn trim(&self, input: &Path, start: f64, end: f64, output: &Path) -> Result<(), RunnerFailure> {
        let status = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(["-ss", &seconds_arg(start), "-to", &seconds_arg(end), "-i"])
            .arg(input)
            .args(["-c", "copy"])
            .arg(output)
            .status()
            .map_err(command_failure)?;
        status.success().then_some(()).ok_or(RunnerFailure::Failed)
    }

    fn concat(&self, manifest: &Path, output: &Path) -> Result<(), RunnerFailure> {
        let status = Command::new(&self.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
            ])
            .arg(manifest)
            .args(["-map", "0", "-c", "copy", "-movflags", "+faststart"])
            .arg(output)
            .status()
            .map_err(command_failure)?;
        status.success().then_some(()).ok_or(RunnerFailure::Failed)
    }
}

/// `%.6f`-formatted seconds for `-ss`/`-to`: six decimals is far finer
/// than the 1ms re-validation tolerance while staying an ASCII float
/// ffmpeg parses unambiguously (no locale-dependent formatting).
fn seconds_arg(seconds: f64) -> String {
    format!("{seconds:.6}")
}

fn command_failure(error: io::Error) -> RunnerFailure {
    if error.kind() == io::ErrorKind::NotFound {
        RunnerFailure::Unavailable
    } else {
        RunnerFailure::Failed
    }
}
