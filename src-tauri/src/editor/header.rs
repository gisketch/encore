use chrono::{Local, NaiveDate, TimeZone};
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path, time::UNIX_EPOCH};

/// Header data for the Editor window: everything the mockup's title row and
/// spec line need, read straight from the bundle rather than assumed.
/// `width`/`height` and the mono spec line derived from them are `None`
/// when `metadata.json` never recorded a geometry (older/degraded
/// bundles) — the frontend omits that segment rather than guessing.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditorHeader {
    pub id: String,
    /// "Today, 4:32 PM" / "Yesterday, 9:01 AM" / "Mon, Aug 10, 9:01 AM" —
    /// built here (not the frontend) so the day-label branching stays out
    /// of the Svelte layer, matching the Library window's existing
    /// grouping convention.
    pub title: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub total_bytes: u64,
    /// Absolute filesystem path to `replay.mp4`, for the frontend to hand
    /// to `convertFileSrc`.
    pub video_path: String,
}

pub(crate) fn build(destination: &Path, id: &str) -> Result<EditorHeader, String> {
    let bundle = crate::library::resolve_bundle_dir(destination, id)?;
    let replay_file = bundle.join(crate::packager::REPLAY_FILENAME);
    let total_bytes = fs::metadata(&replay_file)
        .map(|info| info.len())
        .map_err(|_| "editor_replay_missing".to_string())?;
    let metadata = read_metadata(&bundle);
    let saved_at_unix_ms = metadata
        .as_ref()
        .and_then(|value| value["createdAtUnixMs"].as_u64())
        .unwrap_or_else(|| folder_modified_unix_ms(&bundle));
    let width = geometry_field(metadata.as_ref(), "width");
    let height = geometry_field(metadata.as_ref(), "height");
    Ok(EditorHeader {
        id: id.to_string(),
        title: title(saved_at_unix_ms),
        width,
        height,
        total_bytes,
        video_path: replay_file.to_string_lossy().into_owned(),
    })
}

fn geometry_field(metadata: Option<&Value>, field: &str) -> Option<u32> {
    metadata
        .and_then(|value| value["capture"]["geometry"][field].as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

fn read_metadata(bundle_directory: &Path) -> Option<Value> {
    let bytes = fs::read(bundle_directory.join(crate::packager::METADATA_FILENAME)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn folder_modified_unix_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|info| info.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|since_epoch| u64::try_from(since_epoch.as_millis()).ok())
        .unwrap_or(0)
}

/// "Today"/"Yesterday"/"Mon, Aug 10" plus a time-of-day, e.g.
/// "Today, 4:32 PM". Deliberately a standalone copy of
/// `library::group`'s day-label logic rather than a shared export: that
/// module is already committed and the harness rejects any complexity
/// increase in tracked files, so a small, independent duplicate here is
/// the smaller change.
fn title(saved_at_unix_ms: u64) -> String {
    let today = Local::now().date_naive();
    let date = local_date(saved_at_unix_ms);
    let day = if date == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".to_string()
    } else {
        date.format("%a, %b %-d").to_string()
    };
    format!("{day}, {}", local_time(saved_at_unix_ms))
}

fn local_date(saved_at_unix_ms: u64) -> NaiveDate {
    local_datetime(saved_at_unix_ms).date_naive()
}

fn local_time(saved_at_unix_ms: u64) -> String {
    local_datetime(saved_at_unix_ms)
        .format("%-I:%M %p")
        .to_string()
}

fn local_datetime(saved_at_unix_ms: u64) -> chrono::DateTime<Local> {
    i64::try_from(saved_at_unix_ms)
        .ok()
        .and_then(|millis| Local.timestamp_millis_opt(millis).single())
        .unwrap_or_else(Local::now)
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
                "encore-editor-header-{label}-{}-{id}",
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
    fn reads_geometry_and_size_from_a_real_bundle() {
        let root = TestDirectory::new("full");
        let metadata = r#"{"createdAtUnixMs":1723536000000,"capture":{"geometry":{"width":1920,"height":1080}}}"#;
        write_bundle(
            root.path(),
            "Encore Replay A",
            Some(metadata),
            b"video-bytes",
        );

        let header = build(root.path(), "Encore Replay A").unwrap();

        assert_eq!(header.width, Some(1920));
        assert_eq!(header.height, Some(1080));
        assert_eq!(header.total_bytes, "video-bytes".len() as u64);
        assert_eq!(
            header.video_path,
            root.path()
                .join("Encore Replay A")
                .join("replay.mp4")
                .to_string_lossy()
        );
    }

    #[test]
    fn missing_or_corrupt_metadata_omits_geometry_instead_of_failing() {
        let root = TestDirectory::new("degraded");
        write_bundle(root.path(), "Encore Replay B", Some("not json"), b"video");

        let header = build(root.path(), "Encore Replay B").unwrap();

        assert_eq!(header.width, None);
        assert_eq!(header.height, None);
        assert!(!header.title.is_empty());
    }

    #[test]
    fn a_missing_replay_file_is_rejected() {
        let root = TestDirectory::new("missing");
        fs::create_dir_all(root.path().join("Encore Replay C")).unwrap();

        assert!(build(root.path(), "Encore Replay C").is_err());
    }

    #[test]
    fn an_invalid_id_is_rejected_before_touching_the_filesystem() {
        let root = TestDirectory::new("guard");

        assert!(build(root.path(), "../escape").is_err());
    }
}
