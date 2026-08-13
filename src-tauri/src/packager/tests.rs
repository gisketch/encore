mod sidecars;
mod test_support;

use super::{
    display_name, BundleMediaDescription, BundleStreamDescription, PackageRequest, ReplayPackager,
    RunnerFailure,
};
use crate::retention::RollingStore;
use std::{
    collections::VecDeque,
    fs,
    sync::{Arc, Mutex},
};
use test_support::{
    capture_facts, h264_media, store_with_segments, FailingMetadataFileSystem, FakeRunner,
    TestDirectory,
};

#[test]
fn compatible_segments_publish_one_complete_schema_v1_bundle() {
    let segments = TestDirectory::new("segments");
    let destination = TestDirectory::new("destination");
    let store = store_with_segments(&segments, &["display:1", "display:1"]);
    let lease = store.lease().unwrap();
    let runner = Arc::new(FakeRunner::compatible(2));
    let packager = ReplayPackager::new(runner.clone());
    let request = PackageRequest {
        replay_id: "replay-19",
        created_at_unix_ms: 1_725_000_000_000,
        lease: &lease,
        capture: capture_facts(),
    };

    let saved = packager.package(&request, destination.path()).unwrap();

    assert!(saved.replay_file.is_file());
    assert_eq!(fs::read(&saved.replay_file).unwrap(), b"golden-h264-mp4");
    assert!(saved.metadata_file.is_file());
    assert_eq!(fs::read_dir(&saved.bundle_directory).unwrap().count(), 2);
    assert_eq!(
        saved
            .bundle_directory
            .file_name()
            .unwrap()
            .to_string_lossy(),
        saved.display_name
    );
    assert_eq!(saved.id, "replay-19");
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&saved.metadata_file).unwrap()).unwrap();
    assert_eq!(metadata["schemaVersion"], 1);
    assert_eq!(metadata["replayId"], "replay-19");
    assert_eq!(metadata["evidence"]["segmentCount"], 2);
    assert_eq!(metadata["evidence"]["inputBytes"], 26);
    assert_eq!(metadata["output"]["videoFilename"], "replay.mp4");
    let metadata = serde_json::to_string(&metadata).unwrap();
    assert!(!metadata.contains("display:1"));
    assert!(!metadata.contains("segment-"));
    let manifest = runner.manifest.lock().unwrap().clone().unwrap();
    assert!(manifest.contains("segment-10000-000000.mp4"));
    assert!(manifest.contains("segment-20000-000001.mp4"));
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 1);
    assert!(segments.path().join("segment-10000-000000.mp4").exists());
}

#[test]
fn source_or_media_mismatch_fails_without_consuming_the_caller_lease() {
    let segments = TestDirectory::new("incompatible");
    let destination = TestDirectory::new("destination");
    let store = store_with_segments(&segments, &["display:1", "display:2"]);
    let lease = store.lease().unwrap();
    let packager = ReplayPackager::new(Arc::new(FakeRunner::compatible(2)));
    let request = PackageRequest {
        replay_id: "replay-20",
        created_at_unix_ms: 1_725_000_000_000,
        lease: &lease,
        capture: capture_facts(),
    };

    assert_eq!(
        packager.package(&request, destination.path()),
        Err("export_incompatible_segments".into())
    );
    assert!(segments.path().join("segment-10000-000000.mp4").exists());
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
}

#[test]
fn recovered_segments_with_unknown_source_ids_package_when_media_matches() {
    let segments = TestDirectory::new("recovered-segments");
    let destination = TestDirectory::new("recovered-destination");
    let original = store_with_segments(&segments, &["display:1", "display:1"]);
    drop(original);
    let recovered = RollingStore::open(segments.path().to_owned(), 10).unwrap();
    let lease = recovered.lease().unwrap();
    assert!(lease
        .segments()
        .iter()
        .all(|segment| segment.source_id().is_none()));
    let packager = ReplayPackager::new(Arc::new(FakeRunner::compatible(2)));

    let saved = packager
        .package(
            &PackageRequest {
                replay_id: "recovered-replay",
                created_at_unix_ms: 1_725_000_000_000,
                lease: &lease,
                capture: capture_facts(),
            },
            destination.path(),
        )
        .unwrap();

    assert!(saved.replay_file.is_file());
}

#[test]
fn codec_geometry_or_stream_layout_mismatch_fails_before_concat() {
    let segments = TestDirectory::new("media-mismatch");
    let destination = TestDirectory::new("destination");
    let store = store_with_segments(&segments, &["display:1", "display:1"]);
    let lease = store.lease().unwrap();
    let changed_geometry = BundleMediaDescription {
        streams: vec![BundleStreamDescription {
            width: Some(1280),
            ..h264_media().streams.remove(0)
        }],
    };
    let runner = FakeRunner {
        inspections: Mutex::new(VecDeque::from([Ok(h264_media()), Ok(changed_geometry)])),
        concat_result: Ok(()),
        manifest: Mutex::new(None),
    };
    let packager = ReplayPackager::new(Arc::new(runner));
    let request = PackageRequest {
        replay_id: "replay-20-media",
        created_at_unix_ms: 1_725_000_000_000,
        lease: &lease,
        capture: capture_facts(),
    };

    assert_eq!(
        packager.package(&request, destination.path()),
        Err("export_incompatible_segments".into())
    );
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
}

#[test]
fn sidecar_failures_are_typed_and_leave_no_partial_bundle() {
    let segments = TestDirectory::new("runner-failure");
    let destination = TestDirectory::new("destination");
    let store = store_with_segments(&segments, &["display:1"]);
    let lease = store.lease().unwrap();
    let request = PackageRequest {
        replay_id: "replay-21",
        created_at_unix_ms: 1_725_000_000_000,
        lease: &lease,
        capture: capture_facts(),
    };
    let missing = ReplayPackager::new(Arc::new(FakeRunner::failing_inspection(
        RunnerFailure::Unavailable,
    )));
    assert_eq!(
        missing.package(&request, destination.path()),
        Err("export_ffmpeg_unavailable".into())
    );

    let failed_concat = FakeRunner {
        inspections: Mutex::new(VecDeque::from([Ok(h264_media())])),
        concat_result: Err(RunnerFailure::Failed),
        manifest: Mutex::new(None),
    };
    let packager = ReplayPackager::new(Arc::new(failed_concat));
    assert_eq!(
        packager.package(&request, destination.path()),
        Err("export_concat_failed".into())
    );
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
}

#[test]
fn collisions_get_a_suffix_without_overwriting_a_completed_bundle() {
    let segments = TestDirectory::new("collision-segments");
    let destination = TestDirectory::new("collision-destination");
    let store = store_with_segments(&segments, &["display:1"]);
    let lease = store.lease().unwrap();
    let created = 1_725_000_000_000;
    let original = destination.path().join(display_name(created, 0).unwrap());
    fs::create_dir(&original).unwrap();
    fs::write(original.join("keep.txt"), b"existing evidence").unwrap();
    let packager = ReplayPackager::new(Arc::new(FakeRunner::compatible(1)));
    let request = PackageRequest {
        replay_id: "replay-22",
        created_at_unix_ms: created,
        lease: &lease,
        capture: capture_facts(),
    };

    let saved = packager.package(&request, destination.path()).unwrap();

    assert_eq!(saved.display_name, display_name(created, 1).unwrap());
    assert_eq!(
        fs::read(original.join("keep.txt")).unwrap(),
        b"existing evidence"
    );
}

#[test]
fn injected_metadata_write_failure_cleans_the_hidden_workspace() {
    let segments = TestDirectory::new("filesystem-segments");
    let destination = TestDirectory::new("filesystem-destination");
    let store = store_with_segments(&segments, &["display:1"]);
    let lease = store.lease().unwrap();
    let packager = ReplayPackager::with_files(
        Arc::new(FakeRunner::compatible(1)),
        Arc::new(FailingMetadataFileSystem),
    );
    let request = PackageRequest {
        replay_id: "replay-23",
        created_at_unix_ms: 1_725_000_000_000,
        lease: &lease,
        capture: capture_facts(),
    };

    assert_eq!(
        packager.package(&request, destination.path()),
        Err("export_metadata_failed".into())
    );
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
}

#[test]
fn next_export_removes_only_dead_generated_partial_workspaces_and_locks() {
    let segments = TestDirectory::new("stale-workspace-segments");
    let destination = TestDirectory::new("stale-workspace-destination");
    let stale = destination
        .path()
        .join(".Encore Replay 2026-08-12 10.20.30.partial-999999-1");
    let stale_lock = destination
        .path()
        .join(".Encore Replay 2026-08-12 10.20.30.lock-999999");
    let unrelated = destination.path().join(".Encore Replay notes.lock-999999");
    fs::create_dir(&stale).unwrap();
    fs::write(stale.join("replay.mp4"), b"partial").unwrap();
    fs::write(&stale_lock, b"lock").unwrap();
    fs::write(&unrelated, b"keep").unwrap();
    let store = store_with_segments(&segments, &["display:1"]);
    let lease = store.lease().unwrap();
    let packager = ReplayPackager::new(Arc::new(FakeRunner::compatible(1)));

    packager
        .package(
            &PackageRequest {
                replay_id: "cleanup-replay",
                created_at_unix_ms: 1_725_000_000_000,
                lease: &lease,
                capture: capture_facts(),
            },
            destination.path(),
        )
        .unwrap();

    assert!(!stale.exists());
    assert!(!stale_lock.exists());
    assert_eq!(fs::read(&unrelated).unwrap(), b"keep");
}
