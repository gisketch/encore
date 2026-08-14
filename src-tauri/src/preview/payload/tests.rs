use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "encore-preview-payload-{label}-{}-{id}",
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
fn reports_name_duration_size_and_video_path_from_a_real_bundle() {
    let root = TestDirectory::new("full");
    let metadata = r#"{"createdAtUnixMs":1723536000000,"evidence":{"startUnixMs":1723535940000,"endUnixMs":1723536000000}}"#;
    write_bundle(
        root.path(),
        "Encore Replay A",
        Some(metadata),
        b"video-bytes",
    );

    let payload = build(root.path(), "Encore Replay A").unwrap();

    assert_eq!(payload.id, "Encore Replay A");
    assert_eq!(payload.duration_seconds, Some(60));
    assert_eq!(payload.total_bytes, "video-bytes".len() as u64);
    assert_eq!(
        payload.video_path,
        root.path()
            .join("Encore Replay A")
            .join("replay.mp4")
            .to_string_lossy()
    );
    assert!(!payload.display_name.is_empty());
}

/// The display name is not a second opinion: it is exactly the title the
/// Editor header builds for the same bundle.
#[test]
fn the_display_name_matches_the_editor_header_title() {
    let root = TestDirectory::new("title");
    let metadata =
        r#"{"createdAtUnixMs":1723536000000,"evidence":{"startUnixMs":0,"endUnixMs":5000}}"#;
    write_bundle(root.path(), "Encore Replay B", Some(metadata), b"video");

    let payload = build(root.path(), "Encore Replay B").unwrap();
    let header = crate::editor::header(root.path(), "Encore Replay B").unwrap();

    assert_eq!(payload.display_name, header.title);
}

#[test]
fn missing_or_corrupt_metadata_omits_duration_instead_of_inventing_it() {
    let root = TestDirectory::new("degraded");
    write_bundle(root.path(), "Encore Replay C", Some("not json"), b"video");
    write_bundle(root.path(), "Encore Replay D", None, b"video");
    write_bundle(
        root.path(),
        "Encore Replay E",
        Some(r#"{"createdAtUnixMs":1723536000000}"#),
        b"video",
    );

    for id in ["Encore Replay C", "Encore Replay D", "Encore Replay E"] {
        let payload = build(root.path(), id).unwrap();
        assert_eq!(
            payload.duration_seconds, None,
            "expected {id} to omit duration"
        );
        assert_eq!(payload.total_bytes, "video".len() as u64);
    }
}

#[test]
fn an_id_outside_the_destination_is_rejected_with_a_stable_error_code() {
    let root = TestDirectory::new("guard");

    for id in ["..", ".", "", "../escape", "a/b", "a\\b", "/etc/passwd"] {
        assert_eq!(
            build(root.path(), id).unwrap_err(),
            "library_invalid_id",
            "expected {id:?} to be rejected"
        );
    }
}

#[test]
fn a_bundle_without_a_replay_file_is_rejected() {
    let root = TestDirectory::new("missing");
    fs::create_dir_all(root.path().join("Encore Replay F")).unwrap();

    assert_eq!(
        build(root.path(), "Encore Replay F").unwrap_err(),
        "editor_replay_missing"
    );
}
