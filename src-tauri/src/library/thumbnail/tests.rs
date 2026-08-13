use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "encore-thumbnail-{label}-{}-{id}",
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

fn write_bundle(root: &Path, name: &str, video_bytes: &[u8]) -> PathBuf {
    let bundle = root.join(name);
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("replay.mp4"), video_bytes).unwrap();
    bundle
}

/// Extractor whose `extract` outcome and call count are both controllable,
/// so tests can assert the failed-marker short-circuit really stops it
/// from running twice.
struct FakeExtractor {
    succeeds: bool,
    calls: std::sync::atomic::AtomicU32,
}

impl FakeExtractor {
    fn new(succeeds: bool) -> Self {
        Self {
            succeeds,
            calls: std::sync::atomic::AtomicU32::new(0),
        }
    }

    fn call_count(&self) -> u32 {
        self.calls.load(Ordering::Relaxed)
    }
}

impl ThumbnailExtractor for FakeExtractor {
    fn extract(&self, _input: &Path, output: &Path) -> Result<(), ThumbnailFailure> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.succeeds {
            fs::write(output, b"fake-jpeg-bytes").unwrap();
            Ok(())
        } else {
            Err(ThumbnailFailure::Unavailable)
        }
    }
}

#[test]
fn cache_key_is_stable_for_the_same_file_and_changes_when_mtime_moves() {
    let replay = Path::new("/exports/Encore Replay/replay.mp4");

    let first = cache_key(replay, 1024, 1_723_536_000_000);
    let repeat = cache_key(replay, 1024, 1_723_536_000_000);
    let touched = cache_key(replay, 1024, 1_723_536_100_000);

    assert_eq!(first, repeat);
    assert_ne!(first, touched);
}

#[test]
fn cache_key_changes_when_size_moves_even_with_the_same_mtime() {
    let replay = Path::new("/exports/Encore Replay/replay.mp4");

    let smaller = cache_key(replay, 1024, 1_723_536_000_000);
    let larger = cache_key(replay, 2048, 1_723_536_000_000);

    assert_ne!(smaller, larger);
}

#[test]
fn generates_on_miss_and_reads_from_cache_on_a_second_call() {
    let destination = TestDirectory::new("dest-hit");
    let cache = TestDirectory::new("cache-hit");
    write_bundle(destination.path(), "Encore Replay A", b"video-bytes");
    let extractor = FakeExtractor::new(true);

    let first = thumbnail_bytes(
        destination.path(),
        cache.path(),
        "Encore Replay A",
        &extractor,
    )
    .unwrap();
    let second = thumbnail_bytes(
        destination.path(),
        cache.path(),
        "Encore Replay A",
        &extractor,
    )
    .unwrap();

    assert_eq!(first, b"fake-jpeg-bytes".to_vec());
    assert_eq!(second, first);
    assert_eq!(
        extractor.call_count(),
        1,
        "second call should hit the cache, not re-extract"
    );
}

#[test]
fn a_failed_extraction_writes_a_marker_and_never_retries_for_that_key() {
    let destination = TestDirectory::new("dest-fail");
    let cache = TestDirectory::new("cache-fail");
    write_bundle(destination.path(), "Encore Replay B", b"corrupt-video");
    let extractor = FakeExtractor::new(false);

    let first = thumbnail_bytes(
        destination.path(),
        cache.path(),
        "Encore Replay B",
        &extractor,
    );
    let second = thumbnail_bytes(
        destination.path(),
        cache.path(),
        "Encore Replay B",
        &extractor,
    );

    assert!(first.is_err());
    assert!(second.is_err());
    assert_eq!(
        extractor.call_count(),
        1,
        "a failed key must short-circuit, not retry in a loop"
    );
}

#[test]
fn nothing_is_written_next_to_the_bundle_only_under_the_cache_dir() {
    let destination = TestDirectory::new("dest-isolation");
    let cache = TestDirectory::new("cache-isolation");
    let bundle = write_bundle(destination.path(), "Encore Replay C", b"video-bytes");
    let extractor = FakeExtractor::new(true);

    thumbnail_bytes(
        destination.path(),
        cache.path(),
        "Encore Replay C",
        &extractor,
    )
    .unwrap();

    let bundle_entries: Vec<_> = fs::read_dir(&bundle)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(bundle_entries, vec![std::ffi::OsString::from("replay.mp4")]);
    assert!(cache.path().join(THUMBNAIL_SUBDIR).is_dir());
}

#[test]
fn an_invalid_id_is_rejected_before_touching_the_filesystem() {
    let destination = TestDirectory::new("dest-guard");
    let cache = TestDirectory::new("cache-guard");
    let extractor = FakeExtractor::new(true);

    let result = thumbnail_bytes(destination.path(), cache.path(), "../escape", &extractor);

    assert!(result.is_err());
    assert_eq!(extractor.call_count(), 0);
}

/// Real end-to-end smoke against the bundled ffmpeg sidecar: generates a
/// tiny synthetic MP4 with ffmpeg's `lavfi` test source, then extracts a
/// real thumbnail frame from it. Run explicitly (`cargo test -- --ignored`)
/// since it shells out to a real binary rather than a fake.
#[test]
#[ignore]
fn real_ffmpeg_sidecar_extracts_a_frame_from_a_tiny_generated_clip() {
    let ffmpeg = crate::packager::current_sidecar_path("ffmpeg")
        .expect("ffmpeg sidecar must be present (run npm run prepare:ffmpeg-sidecars)");
    let destination = TestDirectory::new("dest-real");
    let cache = TestDirectory::new("cache-real");
    let bundle = destination.path().join("Encore Replay Real");
    fs::create_dir_all(&bundle).unwrap();
    let replay = bundle.join("replay.mp4");
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
        .arg("testsrc=duration=2:size=64x64:rate=10")
        .args(["-frames:v", "20"])
        .arg(&replay)
        .status()
        .unwrap();
    assert!(status.success(), "failed to synthesize a test clip");

    let extractor = ProcessThumbnailExtractor::new(ffmpeg);
    let bytes = thumbnail_bytes(
        destination.path(),
        cache.path(),
        "Encore Replay Real",
        &extractor,
    )
    .unwrap();

    assert!(!bytes.is_empty());
    assert_eq!(&bytes[..2], &[0xFF, 0xD8], "expected a JPEG signature");
}
