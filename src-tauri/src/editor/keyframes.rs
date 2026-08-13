use serde::Serialize;
use std::{path::Path, path::PathBuf, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbeFailure {
    Unavailable,
}

/// Keyframe timestamps and total duration for the Editor's timeline —
/// trim handles snap to `seconds` so every export the editor produces stays
/// stream-copy lossless, per the spec's grilled "lossless-only cutting"
/// decision.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditorKeyframes {
    pub seconds: Vec<f64>,
    pub duration_seconds: f64,
}

/// Shells out to ffprobe for the two raw CSV outputs `probe` parses. A
/// trait so tests exercise the pure parsers against captured fixture
/// strings instead of a real process.
pub(crate) trait KeyframeProbe: Send + Sync {
    fn packet_csv(&self, input: &Path) -> Result<String, ProbeFailure>;
    fn duration_csv(&self, input: &Path) -> Result<String, ProbeFailure>;
}

pub(crate) struct ProcessKeyframeProbe {
    ffprobe: PathBuf,
}

impl ProcessKeyframeProbe {
    pub(crate) fn new(ffprobe: PathBuf) -> Self {
        Self { ffprobe }
    }

    fn run(&self, args: &[&str], input: &Path) -> Result<String, ProbeFailure> {
        let output = Command::new(&self.ffprobe)
            .args(args)
            .arg(input)
            .output()
            .map_err(|_| ProbeFailure::Unavailable)?;
        if !output.status.success() {
            return Err(ProbeFailure::Unavailable);
        }
        String::from_utf8(output.stdout).map_err(|_| ProbeFailure::Unavailable)
    }
}

impl KeyframeProbe for ProcessKeyframeProbe {
    /// `packet=pts_time,flags` (rather than `-skip_frame nokey
    /// -show_entries frame=...`) — verified directly against both a
    /// Homebrew ffprobe and this project's bundled sidecar binary; the
    /// `skip_frame`+`frame` combination silently returned empty
    /// `pts_time` values on the bundled static build, while packet flags
    /// worked identically on both.
    fn packet_csv(&self, input: &Path) -> Result<String, ProbeFailure> {
        self.run(
            &[
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "packet=pts_time,flags",
                "-of",
                "csv=print_section=0",
            ],
            input,
        )
    }

    fn duration_csv(&self, input: &Path) -> Result<String, ProbeFailure> {
        self.run(
            &[
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "csv=print_section=0",
            ],
            input,
        )
    }
}

/// Parses `ffprobe -show_entries packet=pts_time,flags -of
/// csv=print_section=0` output into ascending, deduplicated keyframe
/// timestamps. Each line is `pts_time,flags`; a keyframe packet's flags
/// field starts with `K`. Packets are listed in decode order (not
/// presentation order), so the result is sorted rather than assumed
/// monotonic; a malformed line is skipped rather than failing the whole
/// parse.
pub(crate) fn parse_keyframe_seconds(csv: &str) -> Vec<f64> {
    let mut seconds: Vec<f64> = csv
        .lines()
        .filter_map(|line| {
            let (pts, flags) = line.split_once(',')?;
            if !flags.starts_with('K') {
                return None;
            }
            pts.trim().parse::<f64>().ok()
        })
        .collect();
    seconds.sort_by(f64::total_cmp);
    seconds.dedup();
    seconds
}

/// Parses `ffprobe -show_entries format=duration -of csv=print_section=0`
/// output (a single numeric line) into seconds.
pub(crate) fn parse_duration_seconds(csv: &str) -> Option<f64> {
    csv.lines().next()?.trim().parse::<f64>().ok()
}

pub(crate) fn probe(input: &Path, probe: &dyn KeyframeProbe) -> Result<EditorKeyframes, String> {
    let packets = probe
        .packet_csv(input)
        .map_err(|_| "editor_keyframes_failed".to_string())?;
    let duration_csv = probe
        .duration_csv(input)
        .map_err(|_| "editor_keyframes_failed".to_string())?;
    let duration_seconds = parse_duration_seconds(&duration_csv)
        .ok_or_else(|| "editor_keyframes_failed".to_string())?;
    Ok(EditorKeyframes {
        seconds: parse_keyframe_seconds(&packets),
        duration_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured verbatim from `ffprobe -v error -select_streams v:0
    /// -show_entries packet=pts_time,flags -of csv=print_section=0` against
    /// a synthetic 3s/30fps clip encoded with a 1s keyframe interval (this
    /// project's `keyframe_interval_seconds` from `packager::model`).
    const PACKET_CSV_FIXTURE: &str = "0.000000,K__\n\
0.133333,___\n\
0.066667,___\n\
0.033333,___\n\
0.100000,___\n\
1.000000,K__\n\
1.133333,___\n\
2.000000,K__\n\
2.966667,___\n";

    #[test]
    fn extracts_only_keyframe_timestamps_sorted_ascending() {
        let seconds = parse_keyframe_seconds(PACKET_CSV_FIXTURE);

        assert_eq!(seconds, vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn skips_lines_that_do_not_parse_as_a_number() {
        let csv = "N/A,K__\n1.500000,K__\nnot-a-line\n";

        assert_eq!(parse_keyframe_seconds(csv), vec![1.5]);
    }

    #[test]
    fn empty_input_yields_no_keyframes() {
        assert!(parse_keyframe_seconds("").is_empty());
    }

    #[test]
    fn parses_a_single_line_duration() {
        assert_eq!(parse_duration_seconds("3.000000\n"), Some(3.0));
    }

    #[test]
    fn blank_duration_output_is_none() {
        assert_eq!(parse_duration_seconds(""), None);
        assert_eq!(parse_duration_seconds("N/A\n"), None);
    }

    struct FakeProbe {
        packets: &'static str,
        duration: &'static str,
    }

    impl KeyframeProbe for FakeProbe {
        fn packet_csv(&self, _input: &Path) -> Result<String, ProbeFailure> {
            Ok(self.packets.to_string())
        }

        fn duration_csv(&self, _input: &Path) -> Result<String, ProbeFailure> {
            Ok(self.duration.to_string())
        }
    }

    struct FailingProbe;

    impl KeyframeProbe for FailingProbe {
        fn packet_csv(&self, _input: &Path) -> Result<String, ProbeFailure> {
            Err(ProbeFailure::Unavailable)
        }

        fn duration_csv(&self, _input: &Path) -> Result<String, ProbeFailure> {
            Err(ProbeFailure::Unavailable)
        }
    }

    #[test]
    fn probe_combines_keyframes_and_duration() {
        let fake = FakeProbe {
            packets: PACKET_CSV_FIXTURE,
            duration: "3.000000\n",
        };

        let result = probe(Path::new("/tmp/does-not-matter.mp4"), &fake).unwrap();

        assert_eq!(result.seconds, vec![0.0, 1.0, 2.0]);
        assert_eq!(result.duration_seconds, 3.0);
    }

    #[test]
    fn probe_failure_surfaces_a_stable_error_code() {
        let result = probe(Path::new("/tmp/does-not-matter.mp4"), &FailingProbe);

        assert_eq!(result, Err("editor_keyframes_failed".to_string()));
    }

    /// Real end-to-end smoke against the bundled ffprobe sidecar: generates
    /// a tiny synthetic MP4 (h264, 1s keyframe interval) with ffmpeg's
    /// `lavfi` test source, then probes it for real. Run explicitly
    /// (`cargo test -- --ignored`) since it shells out to real binaries.
    #[test]
    #[ignore]
    fn real_ffprobe_sidecar_reads_keyframes_from_a_tiny_generated_clip() {
        let ffmpeg = crate::packager::current_sidecar_path("ffmpeg")
            .expect("ffmpeg sidecar must be present (run npm run prepare:ffmpeg-sidecars)");
        let ffprobe = crate::packager::current_sidecar_path("ffprobe")
            .expect("ffprobe sidecar must be present (run npm run prepare:ffmpeg-sidecars)");
        let directory = std::env::temp_dir().join(format!(
            "encore-editor-keyframes-real-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let clip = directory.join("clip.mp4");
        let status = Command::new(&ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
            ])
            .arg("testsrc=duration=3:size=64x64:rate=30")
            .args(["-c:v", "libx264", "-g", "30", "-keyint_min", "30"])
            .arg(&clip)
            .status()
            .unwrap();
        assert!(status.success(), "failed to synthesize a test clip");

        let result = probe(&clip, &ProcessKeyframeProbe::new(ffprobe)).unwrap();

        assert!(
            result.seconds.len() >= 3,
            "expected at least one keyframe per second, got {:?}",
            result.seconds
        );
        assert!((result.duration_seconds - 3.0).abs() < 0.5);

        let _ = std::fs::remove_dir_all(&directory);
    }
}
