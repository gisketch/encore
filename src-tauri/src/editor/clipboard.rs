//! "Copy to clipboard" (grilled decision in
//! `docs/specs/2026-08-13-replay-editor.md`): places a FILE REFERENCE on
//! the macOS pasteboard — never raw video bytes — pointing at the most
//! recent export of the current editor session. The frontend already knows
//! that export's path (it just produced it, or exported first if none
//! existed yet); this module's only job is to refuse to hand that path to
//! `osascript` unless it is really inside the current save destination, so
//! a compromised or buggy frontend can never use this command to read an
//! arbitrary file into the pasteboard.

use std::path::{Path, PathBuf};

/// Rejects `path` unless it resolves (symlinks included) to somewhere
/// inside `destination`. Mirrors `library::guard`'s traversal-guard intent,
/// but works on symlink-resolved absolute paths rather than a bare id,
/// since the frontend hands this command a full export path, not an id.
pub(crate) fn guard_within_destination(destination: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical_destination = destination
        .canonicalize()
        .map_err(|_| "clipboard_path_invalid".to_string())?;
    let canonical_path = path
        .canonicalize()
        .map_err(|_| "clipboard_path_invalid".to_string())?;
    if canonical_path.starts_with(&canonical_destination) {
        Ok(canonical_path)
    } else {
        Err("clipboard_path_invalid".to_string())
    }
}

/// Places a file URL reference for `path` on the macOS pasteboard via
/// `osascript` — dependency-free and sufficient for v1 per the ticket; a
/// native objc2/cocoa pasteboard call is future work if osascript's
/// per-call process spawn ever becomes a problem. Left untested here (the
/// guard above is the part worth unit testing); documented per the
/// ticket's "the actual pasteboard call can stay untested" allowance.
pub(crate) fn copy_to_clipboard(path: &Path) -> Result<(), String> {
    let posix = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let script = format!("set the clipboard to POSIX file \"{posix}\"");
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .status()
        .map_err(|_| "clipboard_unavailable".to_string())
        .and_then(|status| {
            status
                .success()
                .then_some(())
                .ok_or_else(|| "clipboard_failed".to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "encore-editor-clipboard-{label}-{}-{id}",
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
    fn accepts_a_file_directly_inside_the_destination() {
        let destination = TestDirectory::new("inside");
        let file = destination.path().join("Encore Replay X (trimmed).gif");
        fs::write(&file, b"gif-bytes").unwrap();

        let guarded = guard_within_destination(destination.path(), &file).unwrap();

        assert_eq!(guarded, file.canonicalize().unwrap());
    }

    #[test]
    fn accepts_a_file_inside_a_bundle_subdirectory() {
        let destination = TestDirectory::new("bundle");
        let bundle = destination.path().join("Encore Replay X (trimmed)");
        fs::create_dir_all(&bundle).unwrap();
        let file = bundle.join("replay.mp4");
        fs::write(&file, b"video-bytes").unwrap();

        assert!(guard_within_destination(destination.path(), &file).is_ok());
    }

    #[test]
    fn rejects_a_file_outside_the_destination() {
        let destination = TestDirectory::new("outside-dest");
        let outside = TestDirectory::new("outside-file");
        let file = outside.path().join("not-an-export.mp4");
        fs::write(&file, b"video-bytes").unwrap();

        let result = guard_within_destination(destination.path(), &file);

        assert_eq!(result.err(), Some("clipboard_path_invalid".to_string()));
    }

    #[test]
    fn rejects_a_missing_file() {
        let destination = TestDirectory::new("missing");
        let missing = destination.path().join("nope.mp4");

        let result = guard_within_destination(destination.path(), &missing);

        assert_eq!(result.err(), Some("clipboard_path_invalid".to_string()));
    }
}
