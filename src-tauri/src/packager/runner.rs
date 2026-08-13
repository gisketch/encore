use super::{BundleMediaDescription, BundleStreamDescription};
use serde::Deserialize;
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerFailure {
    Unavailable,
    Failed,
}

pub(crate) trait FfmpegRunner: Send + Sync {
    fn inspect(&self, input: &Path) -> Result<BundleMediaDescription, RunnerFailure>;
    fn concat(&self, manifest: &Path, output: &Path) -> Result<(), RunnerFailure>;
}

pub(crate) struct ProcessFfmpegRunner {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

impl ProcessFfmpegRunner {
    pub(crate) fn new(ffmpeg: PathBuf, ffprobe: PathBuf) -> Self {
        Self { ffmpeg, ffprobe }
    }
}

impl FfmpegRunner for ProcessFfmpegRunner {
    fn inspect(&self, input: &Path) -> Result<BundleMediaDescription, RunnerFailure> {
        let output = Command::new(&self.ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type,codec_name,width,height,pix_fmt",
                "-of",
                "json",
            ])
            .arg(input)
            .output()
            .map_err(command_failure)?;
        if !output.status.success() {
            return Err(RunnerFailure::Failed);
        }
        let probe: ProbeOutput =
            serde_json::from_slice(&output.stdout).map_err(|_| RunnerFailure::Failed)?;
        Ok(BundleMediaDescription {
            streams: probe
                .streams
                .into_iter()
                .map(|stream| BundleStreamDescription {
                    kind: stream.codec_type,
                    codec: stream.codec_name,
                    width: stream.width,
                    height: stream.height,
                    pixel_format: stream.pix_fmt,
                })
                .collect(),
        })
    }

    fn concat(&self, manifest: &Path, output: &Path) -> Result<(), RunnerFailure> {
        let status = Command::new(&self.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
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
        if status.success() {
            Ok(())
        } else {
            Err(RunnerFailure::Failed)
        }
    }
}

pub(crate) fn current_sidecar_path(name: &str) -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|_| "export_ffmpeg_unavailable")?;
    let path = resolve_sidecar_path(&executable, name)?;
    path.is_file()
        .then_some(path)
        .ok_or_else(|| "export_ffmpeg_unavailable".into())
}

fn resolve_sidecar_path(executable: &Path, name: &str) -> Result<PathBuf, String> {
    if !matches!(name, "ffmpeg" | "ffprobe") {
        return Err("export_ffmpeg_unavailable".into());
    }
    if is_macos_bundle_executable(executable) {
        return executable
            .parent()
            .map(|directory| directory.join(name))
            .ok_or_else(|| "export_ffmpeg_unavailable".into());
    }
    let target = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else {
        return Err("export_ffmpeg_unavailable".into());
    };
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!("{name}-{target}")))
}

fn is_macos_bundle_executable(executable: &Path) -> bool {
    executable
        .parent()
        .zip(executable.parent().and_then(Path::parent))
        .zip(
            executable
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent),
        )
        .is_some_and(|((macos, contents), app)| {
            macos.file_name().is_some_and(|name| name == "MacOS")
                && contents.file_name().is_some_and(|name| name == "Contents")
                && app.extension().is_some_and(|extension| extension == "app")
        })
}

fn command_failure(error: io::Error) -> RunnerFailure {
    if error.kind() == io::ErrorKind::NotFound {
        RunnerFailure::Unavailable
    } else {
        RunnerFailure::Failed
    }
}

#[derive(Deserialize)]
struct ProbeOutput {
    streams: Vec<ProbeStream>,
}

#[derive(Deserialize)]
struct ProbeStream {
    codec_type: String,
    codec_name: String,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_macos_executable_resolves_to_its_bundled_sidecar() {
        let executable = Path::new("/Applications/Encore.app/Contents/MacOS/Encore");
        assert_eq!(
            resolve_sidecar_path(executable, "ffmpeg").unwrap(),
            PathBuf::from("/Applications/Encore.app/Contents/MacOS/ffmpeg")
        );
        assert_eq!(
            resolve_sidecar_path(executable, "ffprobe").unwrap(),
            PathBuf::from("/Applications/Encore.app/Contents/MacOS/ffprobe")
        );
    }

    #[test]
    fn development_executable_keeps_the_target_specific_source_sidecar() {
        let path = resolve_sidecar_path(Path::new("/repo/target/debug/encore"), "ffmpeg").unwrap();
        let target = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        };
        assert_eq!(
            path,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(format!("ffmpeg-{target}"))
        );
    }

    #[test]
    fn sidecar_names_are_fixed() {
        assert_eq!(
            resolve_sidecar_path(Path::new("/repo/target/debug/encore"), "../../open"),
            Err("export_ffmpeg_unavailable".into())
        );
    }
}
