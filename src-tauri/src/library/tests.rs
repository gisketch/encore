use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "encore-library-index-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn index_reports_totals_across_every_group() {
    let root = TestDirectory::new("totals");
    for (name, metadata_json) in [
        (
            "Encore Replay 2026-08-13 09.00.00",
            r#"{"createdAtUnixMs":1723536000000,"evidence":{"startUnixMs":0,"endUnixMs":3000}}"#,
        ),
        (
            "Encore Replay 2026-08-13 10.00.00",
            r#"{"createdAtUnixMs":1723539600000,"evidence":{"startUnixMs":0,"endUnixMs":8000}}"#,
        ),
    ] {
        let bundle = root.path().join(name);
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("replay.mp4"), b"video-bytes").unwrap();
        fs::write(bundle.join("metadata.json"), metadata_json).unwrap();
    }

    let index = index(root.path());

    assert_eq!(index.total_count, 2);
    assert_eq!(
        index.total_bytes,
        index
            .groups
            .iter()
            .flat_map(|group| &group.entries)
            .map(|e| e.total_bytes)
            .sum::<u64>()
    );
    assert_eq!(
        index
            .groups
            .iter()
            .map(|group| group.entries.len())
            .sum::<usize>(),
        2
    );
}

#[test]
fn index_of_a_missing_destination_is_empty() {
    let missing = std::env::temp_dir().join("encore-library-index-missing-destination");
    let _ = fs::remove_dir_all(&missing);

    let index = index(&missing);

    assert_eq!(index.total_count, 0);
    assert!(index.groups.is_empty());
}

#[test]
fn open_replay_file_rejects_traversal_before_touching_disk() {
    let root = TestDirectory::new("open-guard");

    let result = open_replay_file(root.path(), "../escape");

    assert_eq!(result, Err("library_invalid_id".to_string()));
}

#[test]
fn open_replay_file_reports_a_missing_video_without_a_crash() {
    let root = TestDirectory::new("open-missing");
    fs::create_dir_all(root.path().join("Encore Replay 2026-08-13 09.00.00")).unwrap();

    let result = open_replay_file(root.path(), "Encore Replay 2026-08-13 09.00.00");

    assert_eq!(result, Err("library_replay_missing".to_string()));
}
