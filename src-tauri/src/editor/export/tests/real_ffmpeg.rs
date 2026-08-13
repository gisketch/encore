//! Real end-to-end smoke against the bundled ffmpeg/ffprobe sidecars,
//! mirroring `editor::keyframes`'s own real-sidecar test. Run explicitly
//! (`cargo test -- --ignored`) since it shells out to real binaries.

use super::super::{export_trimmed, KeepSegment};
use crate::{
    editor::{export::ProcessExportRunner, keyframes::ProcessKeyframeProbe},
    packager::{current_sidecar_path, FfmpegRunner, ProcessFfmpegRunner, SystemPackageFileSystem},
};
use std::{fs, process::Command};

#[test]
#[ignore]
fn real_ffmpeg_exports_two_kept_ranges_as_a_stream_copy() {
    let ffmpeg = current_sidecar_path("ffmpeg")
        .expect("ffmpeg sidecar must be present (run npm run prepare:ffmpeg-sidecars)");
    let ffprobe = current_sidecar_path("ffprobe")
        .expect("ffprobe sidecar must be present (run npm run prepare:ffmpeg-sidecars)");

    let root =
        std::env::temp_dir().join(format!("encore-editor-export-real-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let bundle = root.join("Encore Replay Real");
    fs::create_dir_all(&bundle).unwrap();

    let clip = bundle.join("replay.mp4");
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
        .arg("testsrc=duration=4:size=64x64:rate=30")
        .args(["-c:v", "libx264", "-g", "30", "-keyint_min", "30"])
        .arg(&clip)
        .status()
        .unwrap();
    assert!(status.success(), "failed to synthesize a test clip");
    fs::write(bundle.join("metadata.json"), r#"{"schemaVersion":1}"#).unwrap();

    let keyframes =
        crate::editor::keyframes::probe(&clip, &ProcessKeyframeProbe::new(ffprobe.clone()))
            .unwrap();
    assert!(
        keyframes.seconds.len() >= 4,
        "expected at least one keyframe per second, got {:?}",
        keyframes.seconds
    );

    // Keep [0, 1) and [2, 3): a real interior cut in the middle.
    let keep_segments = vec![
        KeepSegment {
            start: 0.0,
            end: 1.0,
        },
        KeepSegment {
            start: 2.0,
            end: 3.0,
        },
    ];

    let probe = ProcessKeyframeProbe::new(ffprobe.clone());
    let runner = ProcessExportRunner::new(ffmpeg);
    let files = SystemPackageFileSystem;

    let started = std::time::Instant::now();
    let exported = export_trimmed(
        &root,
        "Encore Replay Real",
        &keep_segments,
        &probe,
        &runner,
        &files,
    )
    .unwrap();
    let elapsed = started.elapsed();

    let output = exported.bundle_directory.join("replay.mp4");
    let inspector = ProcessFfmpegRunner::new(
        current_sidecar_path("ffmpeg").unwrap(),
        current_sidecar_path("ffprobe").unwrap(),
    );
    let exported_probe = ProcessKeyframeProbe::new(current_sidecar_path("ffprobe").unwrap());
    let exported_keyframes = crate::editor::keyframes::probe(&output, &exported_probe).unwrap();

    // Near-instant relative to the source duration proves a stream copy
    // happened rather than a re-encode.
    assert!(
        elapsed.as_secs_f64() < 4.0,
        "export took {:?}, too slow for a stream copy",
        elapsed
    );
    // Duration ~= sum of the two kept ranges (1s + 1s).
    assert!(
        (exported_keyframes.duration_seconds - 2.0).abs() < 0.5,
        "expected ~2s, got {}",
        exported_keyframes.duration_seconds
    );

    // Codec parameters unchanged (stream copy, not a re-encode).
    let source_media = inspector.inspect(&clip).unwrap();
    let output_media = inspector.inspect(&output).unwrap();
    assert_eq!(source_media, output_media);

    let _ = fs::remove_dir_all(&root);
}
