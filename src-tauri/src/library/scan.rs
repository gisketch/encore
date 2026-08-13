use serde_json::Value;
use std::{fs, path::Path, time::UNIX_EPOCH};

/// One bundle folder read off disk, before day-grouping. Metadata that is
/// missing or fails to parse never fails the scan — the entry still
/// appears, just without a duration and with the folder's mtime standing
/// in for the saved-at timestamp, per the spec's "degraded, never an
/// error" contract.
pub(crate) struct ScannedEntry {
    pub id: String,
    pub display_name: String,
    pub saved_at_unix_ms: u64,
    pub duration_seconds: Option<u64>,
    pub total_bytes: u64,
    pub trimmed: bool,
}

/// Scans `destination` for bundle folders, newest-first. Bundle folders are
/// any non-hidden directory directly inside `destination` — hidden entries
/// are skipped so the packager's interrupted-workspace markers
/// (`.Encore Replay ....partial-`, `.lock-`) never show up as replays. A
/// missing or unreadable `destination` yields an empty library rather than
/// an error, matching a fresh install with nothing saved yet.
pub(crate) fn scan(destination: &Path) -> Vec<ScannedEntry> {
    let Ok(entries) = fs::read_dir(destination) else {
        return Vec::new();
    };
    let mut bundles: Vec<ScannedEntry> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| scan_bundle(&entry.path()))
        .collect();
    bundles.sort_by_key(|entry| std::cmp::Reverse(entry.saved_at_unix_ms));
    bundles
}

fn scan_bundle(path: &Path) -> Option<ScannedEntry> {
    let id = path.file_name()?.to_str()?.to_owned();
    if id.starts_with('.') {
        return None;
    }
    let metadata = read_metadata(path);
    let saved_at_unix_ms = metadata
        .as_ref()
        .and_then(|value| value["createdAtUnixMs"].as_u64())
        .unwrap_or_else(|| folder_modified_unix_ms(path));
    let duration_seconds = metadata.as_ref().and_then(duration_from_metadata);
    let trimmed = metadata
        .as_ref()
        .and_then(|value| value["trimmed"].as_bool())
        .unwrap_or(false);
    Some(ScannedEntry {
        display_name: id.clone(),
        id,
        saved_at_unix_ms,
        duration_seconds,
        total_bytes: directory_size(path),
        trimmed,
    })
}

fn read_metadata(bundle_directory: &Path) -> Option<Value> {
    let bytes = fs::read(bundle_directory.join(crate::packager::METADATA_FILENAME)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// The evidence bundle schema (`packager::model::metadata_json`) records an
/// evidence window, not a duration field directly — this derives seconds
/// from it rather than inventing a value the pipeline never recorded.
fn duration_from_metadata(value: &Value) -> Option<u64> {
    let start = value["evidence"]["startUnixMs"].as_u64()?;
    let end = value["evidence"]["endUnixMs"].as_u64()?;
    end.checked_sub(start).map(|elapsed_ms| elapsed_ms / 1000)
}

fn folder_modified_unix_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|info| info.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|since_epoch| u64::try_from(since_epoch.as_millis()).ok())
        .unwrap_or(0)
}

fn directory_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let entry_path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                directory_size(&entry_path)
            } else {
                fs::metadata(&entry_path)
                    .map(|info| info.len())
                    .unwrap_or(0)
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "encore-library-{label}-{}-{id}",
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

    fn write_bundle(root: &Path, name: &str, metadata: Option<&str>, video_bytes: &[u8]) {
        let bundle = root.join(name);
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("replay.mp4"), video_bytes).unwrap();
        if let Some(metadata) = metadata {
            fs::write(bundle.join("metadata.json"), metadata).unwrap();
        }
    }

    #[test]
    fn scans_bundles_with_metadata_newest_first_and_sums_folder_bytes() {
        let root = TestDirectory::new("newest-first");
        let earlier_metadata =
            r#"{"createdAtUnixMs":1000,"evidence":{"startUnixMs":0,"endUnixMs":5000}}"#;
        let later_metadata = r#"{"createdAtUnixMs":2000,"evidence":{"startUnixMs":0,"endUnixMs":12000},"trimmed":true}"#;
        write_bundle(
            root.path(),
            "Encore Replay 2026-08-13 09.00.00",
            Some(earlier_metadata),
            b"earlier-video",
        );
        write_bundle(
            root.path(),
            "Encore Replay 2026-08-13 10.00.00",
            Some(later_metadata),
            b"later-video-longer",
        );

        let scanned = scan(root.path());

        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].id, "Encore Replay 2026-08-13 10.00.00");
        assert_eq!(scanned[0].saved_at_unix_ms, 2000);
        assert_eq!(scanned[0].duration_seconds, Some(12));
        assert!(scanned[0].trimmed);
        assert_eq!(
            scanned[0].total_bytes,
            "later-video-longer".len() as u64 + later_metadata.len() as u64
        );
        assert_eq!(scanned[1].id, "Encore Replay 2026-08-13 09.00.00");
        assert_eq!(scanned[1].duration_seconds, Some(5));
        assert!(!scanned[1].trimmed);
    }

    #[test]
    fn corrupt_or_missing_metadata_degrades_instead_of_erroring() {
        let root = TestDirectory::new("degraded");
        write_bundle(
            root.path(),
            "Encore Replay 2026-08-13 08.00.00",
            Some("not json"),
            b"video",
        );
        write_bundle(
            root.path(),
            "Encore Replay 2026-08-13 07.00.00",
            None,
            b"video",
        );

        let scanned = scan(root.path());

        assert_eq!(scanned.len(), 2);
        for entry in &scanned {
            assert_eq!(entry.duration_seconds, None);
            assert!(!entry.trimmed);
            assert!(entry.saved_at_unix_ms > 0);
        }
    }

    #[test]
    fn hidden_workspace_entries_are_skipped() {
        let root = TestDirectory::new("hidden");
        write_bundle(
            root.path(),
            "Encore Replay 2026-08-13 06.00.00",
            None,
            b"video",
        );
        fs::create_dir_all(
            root.path()
                .join(".Encore Replay 2026-08-13 06.00.00.partial-1-1"),
        )
        .unwrap();

        let scanned = scan(root.path());

        assert_eq!(scanned.len(), 1);
    }

    #[test]
    fn missing_destination_scans_as_empty() {
        let missing = std::env::temp_dir().join("encore-library-does-not-exist");
        let _ = fs::remove_dir_all(&missing);

        assert!(scan(&missing).is_empty());
    }
}
