use super::RollingStore;
use crate::capture::{SegmentBoundary, SegmentRecord};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("encore-retention-test-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn segment(&self, time_ms: u64, sequence: u64) -> SegmentRecord {
        let path = self.0.join(format!("segment-{time_ms}-{sequence:06}.mp4"));
        fs::write(&path, [1_u8; 16]).unwrap();
        SegmentRecord {
            path: path.to_string_lossy().into_owned(),
            source_id: "display:1".into(),
            width: 1920,
            height: 1080,
            byte_size: 16,
            duration_micros: 10_000_000,
            session_start_micros: time_ms * 1_000,
            session_end_micros: (time_ms + 10_000) * 1_000,
            completed_at_unix_ms: time_ms + 10_000,
            boundary: SegmentBoundary::Duration,
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn startup_recovers_completed_segments_and_cleans_crash_files() {
    let directory = TestDirectory::new();
    let first = directory.segment(10_000, 1);
    let second = directory.segment(20_000, 2);
    let partial = directory.path().join("segment-30000-000003.partial.mp4");
    let empty = directory.path().join("segment-40000-000004.mp4");
    let unrelated = directory.path().join("saved-replay.mp4");
    let unrelated_partial = directory.path().join("saved-replay.partial.mp4");
    fs::write(&partial, [1_u8]).unwrap();
    fs::write(&empty, []).unwrap();
    fs::write(&unrelated, [1_u8]).unwrap();
    fs::write(&unrelated_partial, [1_u8]).unwrap();

    let store = RollingStore::open(directory.path().to_owned(), 10).unwrap();

    assert_eq!(store.snapshot().retained_segments, 2);
    assert_eq!(
        store.paths(),
        vec![
            fs::canonicalize(first.path).unwrap(),
            fs::canonicalize(second.path).unwrap()
        ]
    );
    assert!(!partial.exists());
    assert!(!empty.exists());
    assert!(unrelated.exists());
    assert!(unrelated_partial.exists());
}

#[test]
fn five_minute_policy_keeps_one_boundary_segment() {
    let directory = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 5).unwrap();
    for sequence in 0..=40 {
        let record = directory.segment(sequence * 10_000, sequence);
        store.admit(&record).unwrap();
    }

    let paths = store.paths();
    assert_eq!(paths.len(), 30);
    assert!(paths[0].ends_with("segment-110000-000011.mp4"));
    assert!(paths[29].ends_with("segment-400000-000040.mp4"));
}

#[test]
fn ten_minute_policy_keeps_one_boundary_segment() {
    let directory = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 10).unwrap();
    for sequence in 0..=70 {
        let record = directory.segment(sequence * 10_000, sequence);
        store.admit(&record).unwrap();
    }

    let paths = store.paths();
    assert_eq!(paths.len(), 60);
    assert!(paths[0].ends_with("segment-110000-000011.mp4"));
    assert!(paths[59].ends_with("segment-700000-000070.mp4"));
}

#[test]
fn changing_to_five_minutes_prunes_immediately() {
    let directory = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 10).unwrap();
    for sequence in 0..=40 {
        let record = directory.segment(sequence * 10_000, sequence);
        store.admit(&record).unwrap();
    }
    assert_eq!(store.snapshot().retained_segments, 41);

    let snapshot = store.set_minutes(5).unwrap();

    assert_eq!(snapshot.minutes, 5);
    assert_eq!(snapshot.retained_segments, 30);
}

#[test]
fn lease_protects_expired_files_until_release() {
    let directory = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 5).unwrap();
    for sequence in 0..=2 {
        let record = directory.segment(sequence * 10_000, sequence);
        store.admit(&record).unwrap();
    }
    let lease = store.lease().unwrap();
    let oldest = lease.segments()[0].path.clone();
    assert_eq!(lease.segments().len(), 3);
    for sequence in 3..=40 {
        let record = directory.segment(sequence * 10_000, sequence);
        store.admit(&record).unwrap();
    }

    assert!(oldest.exists());
    drop(lease);
    store.prune().unwrap();
    assert!(!oldest.exists());
}

#[test]
fn admission_rejects_files_outside_the_rolling_directory() {
    let directory = TestDirectory::new();
    let outside = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 10).unwrap();
    let record = outside.segment(10_000, 1);

    assert_eq!(
        store.admit(&record),
        Err("retention_segment_invalid".into())
    );
    assert_eq!(
        store.snapshot().error_code.as_deref(),
        Some("retention_segment_invalid")
    );
    let valid = directory.segment(20_000, 2);
    assert_eq!(store.admit(&valid), Err("retention_segment_invalid".into()));
}

#[test]
fn only_five_and_ten_minutes_are_valid() {
    let directory = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 10).unwrap();

    assert_eq!(
        store.set_minutes(7),
        Err("retention_duration_invalid".into())
    );
    assert_eq!(store.snapshot().minutes, 10);
}
