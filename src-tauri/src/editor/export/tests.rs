mod real_ffmpeg;
mod support;

use super::export_trimmed;
use crate::packager::SystemPackageFileSystem;
use std::fs;
use support::{keep, write_source_bundle, FakeExportRunner, FakeProbe, TestDirectory};

#[test]
fn exports_a_single_kept_segment_as_a_trimmed_sibling_bundle() {
    let root = TestDirectory::new("single");
    write_source_bundle(root.path(), "Encore Replay A", b"source-video");
    let runner = FakeExportRunner::new();
    let files = SystemPackageFileSystem;

    let exported = export_trimmed(
        root.path(),
        "Encore Replay A",
        &[keep(1.0, 3.0)],
        &FakeProbe,
        &runner,
        &files,
    )
    .unwrap();

    assert_eq!(
        exported
            .bundle_directory
            .file_name()
            .unwrap()
            .to_string_lossy(),
        "Encore Replay A (trimmed)"
    );
    let replay = exported.bundle_directory.join("replay.mp4");
    assert_eq!(fs::read(&replay).unwrap(), b"trimmed-segment");
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(exported.bundle_directory.join("metadata.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["trimmed"], true);
    assert_eq!(metadata["sourceReplayId"], "Encore Replay A");
    assert_eq!(metadata["keptSegments"][0]["start"], 1.0);
    assert_eq!(metadata["keptSegments"][0]["end"], 3.0);
    assert_eq!(metadata["schemaVersion"], 1); // copied from source metadata
    assert_eq!(*runner.concat_calls.lock().unwrap(), 0); // single segment skips concat
}

#[test]
fn the_source_bundle_is_never_modified() {
    let root = TestDirectory::new("untouched");
    let source = write_source_bundle(root.path(), "Encore Replay B", b"source-video");
    let before_video = fs::read(source.join("replay.mp4")).unwrap();
    let before_metadata = fs::read(source.join("metadata.json")).unwrap();
    let before_video_modified = fs::metadata(source.join("replay.mp4"))
        .unwrap()
        .modified()
        .unwrap();
    let runner = FakeExportRunner::new();
    let files = SystemPackageFileSystem;

    export_trimmed(
        root.path(),
        "Encore Replay B",
        &[keep(0.0, 2.0)],
        &FakeProbe,
        &runner,
        &files,
    )
    .unwrap();

    assert_eq!(fs::read(source.join("replay.mp4")).unwrap(), before_video);
    assert_eq!(
        fs::read(source.join("metadata.json")).unwrap(),
        before_metadata
    );
    assert_eq!(
        fs::metadata(source.join("replay.mp4"))
            .unwrap()
            .modified()
            .unwrap(),
        before_video_modified
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2); // source + trimmed sibling only
}

#[test]
fn two_kept_segments_are_trimmed_in_order_then_concatenated() {
    let root = TestDirectory::new("multi");
    write_source_bundle(root.path(), "Encore Replay C", b"source-video");
    let runner = FakeExportRunner::new();
    let files = SystemPackageFileSystem;

    export_trimmed(
        root.path(),
        "Encore Replay C",
        &[keep(0.0, 1.0), keep(2.0, 3.0)],
        &FakeProbe,
        &runner,
        &files,
    )
    .unwrap();

    let trims = runner.trims.lock().unwrap();
    assert_eq!(trims.len(), 2);
    assert_eq!((trims[0].start, trims[0].end), (0.0, 1.0));
    assert_eq!((trims[1].start, trims[1].end), (2.0, 3.0));
    assert_eq!(*runner.concat_calls.lock().unwrap(), 1);
    let manifest = runner.last_manifest.lock().unwrap().clone().unwrap();
    assert!(manifest.contains("segment-0.mp4"));
    assert!(manifest.contains("segment-1.mp4"));
}

#[test]
fn a_second_export_of_the_same_source_dedupes_its_name() {
    let root = TestDirectory::new("collision");
    write_source_bundle(root.path(), "Encore Replay D", b"source-video");
    let files = SystemPackageFileSystem;

    let first = export_trimmed(
        root.path(),
        "Encore Replay D",
        &[keep(0.0, 1.0)],
        &FakeProbe,
        &FakeExportRunner::new(),
        &files,
    )
    .unwrap();
    let second = export_trimmed(
        root.path(),
        "Encore Replay D",
        &[keep(0.0, 1.0)],
        &FakeProbe,
        &FakeExportRunner::new(),
        &files,
    )
    .unwrap();

    assert_eq!(
        first
            .bundle_directory
            .file_name()
            .unwrap()
            .to_string_lossy(),
        "Encore Replay D (trimmed)"
    );
    assert_eq!(
        second
            .bundle_directory
            .file_name()
            .unwrap()
            .to_string_lossy(),
        "Encore Replay D (trimmed) 1"
    );
}

#[test]
fn an_endpoint_off_any_real_keyframe_is_rejected_before_any_ffmpeg_call() {
    let root = TestDirectory::new("rejected");
    write_source_bundle(root.path(), "Encore Replay E", b"source-video");
    let runner = FakeExportRunner::new();
    let files = SystemPackageFileSystem;

    let result = export_trimmed(
        root.path(),
        "Encore Replay E",
        &[keep(0.5, 3.0)], // 0.5 is not a keyframe or a boundary in FakeProbe
        &FakeProbe,
        &runner,
        &files,
    );

    assert_eq!(result.err(), Some("export_segments_invalid".to_string()));
    assert!(runner.trims.lock().unwrap().is_empty());
    // No partial/lock artifacts leaked into the destination.
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1); // source bundle only
}

#[test]
fn a_trim_failure_leaves_no_partial_bundle_behind() {
    let root = TestDirectory::new("trim-failure");
    write_source_bundle(root.path(), "Encore Replay F", b"source-video");
    let runner = FakeExportRunner::failing();
    let files = SystemPackageFileSystem;

    let result = export_trimmed(
        root.path(),
        "Encore Replay F",
        &[keep(0.0, 1.0)],
        &FakeProbe,
        &runner,
        &files,
    );

    assert_eq!(result.err(), Some("export_trim_failed".to_string()));
    let remaining: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(remaining, vec!["Encore Replay F".to_string()]);
}

#[test]
fn a_missing_replay_id_is_rejected_by_the_shared_traversal_guard() {
    let root = TestDirectory::new("missing");
    let files = SystemPackageFileSystem;

    let result = export_trimmed(
        root.path(),
        "../escape",
        &[keep(0.0, 1.0)],
        &FakeProbe,
        &FakeExportRunner::new(),
        &files,
    );

    assert!(result.is_err());
}
