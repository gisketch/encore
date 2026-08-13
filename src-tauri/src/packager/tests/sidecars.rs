use super::test_support::{capture_facts, TestDirectory};
use crate::{
    capture::{SegmentBoundary, SegmentRecord},
    packager::{current_sidecar_path, PackageRequest, ProcessFfmpegRunner, ReplayPackager},
    retention::RollingStore,
};
use std::{fs, process::Command, sync::Arc};

#[test]
#[ignore = "run `npm run prepare:ffmpeg-sidecars`, then cargo test prepared_sidecars_are_runnable -- --ignored"]
fn prepared_sidecars_are_runnable() {
    for name in ["ffmpeg", "ffprobe"] {
        let sidecar = current_sidecar_path(name).unwrap();
        assert!(Command::new(sidecar)
            .arg("-version")
            .status()
            .unwrap()
            .success());
    }
}

#[test]
#[ignore = "run `npm run prepare:ffmpeg-sidecars`, then cargo test prepared_sidecars_package_h264_golden_media -- --ignored"]
fn prepared_sidecars_package_h264_golden_media() {
    let segments = TestDirectory::new("real-media-segments");
    let destination = TestDirectory::new("real-media-destination");
    let ffmpeg = current_sidecar_path("ffmpeg").unwrap();
    let ffprobe = current_sidecar_path("ffprobe").unwrap();
    let store = RollingStore::open(segments.path().to_owned(), 10).unwrap();
    for index in 0..2_u64 {
        let path = segments.path().join(format!(
            "segment-{}-{index:06}.mp4",
            10_000 + index * 10_000
        ));
        assert!(Command::new(&ffmpeg)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=320x240:r=30",
                "-t",
                "1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+frag_keyframe+empty_moov",
            ])
            .arg(&path)
            .status()
            .unwrap()
            .success());
        store
            .admit(&SegmentRecord {
                path: path.to_string_lossy().into_owned(),
                source_id: "display:1".into(),
                width: 320,
                height: 240,
                byte_size: fs::metadata(&path).unwrap().len(),
                duration_micros: 1_000_000,
                session_start_micros: index * 1_000_000,
                session_end_micros: (index + 1) * 1_000_000,
                completed_at_unix_ms: 10_000 + index * 10_000,
                boundary: SegmentBoundary::Duration,
            })
            .unwrap();
    }
    let lease = store.lease().unwrap();
    let packager = ReplayPackager::new(Arc::new(ProcessFfmpegRunner::new(ffmpeg, ffprobe.clone())));
    let saved = packager
        .package(
            &PackageRequest {
                replay_id: "golden-replay",
                created_at_unix_ms: 1_725_000_000_000,
                lease: &lease,
                capture: capture_facts(),
            },
            destination.path(),
        )
        .unwrap();
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=nw=1",
        ])
        .arg(saved.replay_file)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "codec_name=h264"
    );
}
