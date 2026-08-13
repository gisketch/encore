use super::{guard, thumbnail};
use std::{fs, path::Path};

/// Moves a path to the OS trash. A trait so tests can inject a fake
/// instead of touching the real macOS Trash — mirrors
/// `thumbnail::ThumbnailExtractor` and `packager::FfmpegRunner`'s seams.
pub(crate) trait TrashMover: Send + Sync {
    fn move_to_trash(&self, path: &Path) -> Result<(), String>;
}

/// Production mover: the `trash` crate, which on macOS moves the item
/// through the Finder-recognized Trash rather than deleting it outright —
/// the spec's grilled decision ("moves the whole bundle folder to the
/// macOS Trash ... hard delete is never used").
pub(crate) struct SystemTrashMover;

impl TrashMover for SystemTrashMover {
    fn move_to_trash(&self, path: &Path) -> Result<(), String> {
        trash::delete(path).map_err(|_| "library_delete_failed".to_string())
    }
}

/// Deletes bundle `id` inside `destination`: resolves and validates the id
/// via `guard::resolve_bundle_dir` (rejecting traversal before touching
/// disk, exactly like `open_replay_file`), confirms the bundle still
/// exists, then hands the folder to `mover`. `cache_dir` is optional (a
/// missing app cache dir shouldn't block a delete) and, when present, its
/// matching thumbnail cache entry is forgotten too.
pub(crate) fn delete_bundle(
    destination: &Path,
    cache_dir: Option<&Path>,
    id: &str,
    mover: &dyn TrashMover,
) -> Result<(), String> {
    let bundle = guard::resolve_bundle_dir(destination, id)?;
    if !bundle.is_dir() {
        return Err("library_replay_missing".to_string());
    }
    let replay_file = bundle.join(crate::packager::REPLAY_FILENAME);
    let replay_stat = fs::metadata(&replay_file).ok();
    mover.move_to_trash(&bundle)?;
    forget_cache(cache_dir, &replay_file, replay_stat);
    Ok(())
}

fn forget_cache(cache_dir: Option<&Path>, replay_file: &Path, stat: Option<fs::Metadata>) {
    let Some(cache_dir) = cache_dir else { return };
    let Some(stat) = stat else { return };
    thumbnail::forget(cache_dir, replay_file, &stat);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::{
            atomic::{AtomicU64, Ordering},
            Mutex,
        },
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "encore-library-delete-{label}-{}-{id}",
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

    fn write_bundle(root: &Path, name: &str) -> PathBuf {
        let bundle = root.join(name);
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("replay.mp4"), b"video-bytes").unwrap();
        fs::write(bundle.join("metadata.json"), b"{}").unwrap();
        bundle
    }

    /// Records the path it was asked to move and, to keep the orchestration
    /// test's "removed from destination" assertion meaningful without
    /// touching the real Trash, actually relocates the folder into a
    /// sibling directory standing in for it.
    struct RecordingMover {
        trash_root: PathBuf,
        calls: Mutex<Vec<PathBuf>>,
    }

    impl RecordingMover {
        fn new(trash_root: PathBuf) -> Self {
            fs::create_dir_all(&trash_root).unwrap();
            Self {
                trash_root,
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl TrashMover for RecordingMover {
        fn move_to_trash(&self, path: &Path) -> Result<(), String> {
            self.calls.lock().unwrap().push(path.to_path_buf());
            let destination = self.trash_root.join(path.file_name().unwrap());
            fs::rename(path, destination).map_err(|_| "library_delete_failed".to_string())
        }
    }

    #[test]
    fn rejects_traversal_ids_without_calling_the_mover() {
        let root = TestDirectory::new("guard");
        let trash = TestDirectory::new("guard-trash");
        let mover = RecordingMover::new(trash.path().to_path_buf());

        let result = delete_bundle(root.path(), None, "../escape", &mover);

        assert_eq!(result, Err("library_invalid_id".to_string()));
        assert!(mover.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn reports_a_missing_bundle_without_calling_the_mover() {
        let root = TestDirectory::new("missing");
        let trash = TestDirectory::new("missing-trash");
        let mover = RecordingMover::new(trash.path().to_path_buf());

        let result = delete_bundle(root.path(), None, "nonexistent", &mover);

        assert_eq!(result, Err("library_replay_missing".to_string()));
        assert!(mover.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn moves_an_existing_bundle_out_of_the_destination() {
        let root = TestDirectory::new("existing");
        let trash = TestDirectory::new("existing-trash");
        let bundle_name = "Encore Replay 2026-08-13 09.00.00";
        let bundle = write_bundle(root.path(), bundle_name);
        let mover = RecordingMover::new(trash.path().to_path_buf());

        let result = delete_bundle(root.path(), None, bundle_name, &mover);

        assert!(result.is_ok());
        assert!(
            !bundle.exists(),
            "bundle should be gone from the destination"
        );
        assert!(trash.path().join(bundle_name).is_dir());
        assert_eq!(mover.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn forgets_the_thumbnail_cache_entry_on_success() {
        let root = TestDirectory::new("cache");
        let trash = TestDirectory::new("cache-trash");
        let cache = TestDirectory::new("cache-dir");
        let bundle_name = "Encore Replay 2026-08-13 09.00.00";
        let bundle = write_bundle(root.path(), bundle_name);
        let replay_file = bundle.join("replay.mp4");
        let stat = fs::metadata(&replay_file).unwrap();
        // Pre-populate the cache entry the way `thumbnail::thumbnail_bytes`
        // would, then confirm it's gone after delete.
        let cache_key_extractor = FakeCachePopulatingExtractor;
        let jpeg = thumbnail::thumbnail_bytes(
            root.path(),
            cache.path(),
            bundle_name,
            &cache_key_extractor,
        );
        assert!(jpeg.is_ok());
        let _ = stat;
        let mover = RecordingMover::new(trash.path().to_path_buf());

        let result = delete_bundle(root.path(), Some(cache.path()), bundle_name, &mover);

        assert!(result.is_ok());
        let remaining: Vec<_> = fs::read_dir(cache.path().join("thumbnails"))
            .map(|entries| entries.filter_map(Result::ok).collect())
            .unwrap_or_default();
        assert!(
            remaining.is_empty(),
            "expected the thumbnail cache entry to be forgotten, found {remaining:?}"
        );
    }

    struct FakeCachePopulatingExtractor;

    impl thumbnail::ThumbnailExtractor for FakeCachePopulatingExtractor {
        fn extract(&self, _input: &Path, output: &Path) -> Result<(), thumbnail::ThumbnailFailure> {
            fs::write(output, b"jpeg-bytes").map_err(|_| thumbnail::ThumbnailFailure::Unavailable)
        }
    }

    /// Exercises the real `trash` crate against a disposable fixture. Kept
    /// `#[ignore]` (run explicitly with `cargo test -- --ignored`) since
    /// moving files through the OS Trash can be flaky or unavailable in a
    /// sandboxed/CI environment without a full user session — the
    /// orchestration above already covers the guard/missing/success paths
    /// against an injected mover.
    #[test]
    #[ignore]
    fn real_trash_mover_removes_the_bundle_from_its_destination() {
        let root = TestDirectory::new("real-trash");
        let bundle_name = "Encore Replay 2026-08-13 09.00.00";
        let bundle = write_bundle(root.path(), bundle_name);

        let result = delete_bundle(root.path(), None, bundle_name, &SystemTrashMover);

        assert!(result.is_ok(), "{result:?}");
        assert!(!bundle.exists());
    }
}
