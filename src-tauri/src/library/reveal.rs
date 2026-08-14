//! Revealing one specific bundle in Finder.
//!
//! `CaptureService::reveal_export_folder` opens the *destination folder*,
//! which is the right answer for the Library window's "Open Folder ↗" but
//! not for a surface that already knows which replay it is talking about
//! (the post-save preview). This selects the bundle's replay file with
//! `open -R`, reusing the replay module's one Finder-reveal helper so the
//! `open -R` invocation itself is not duplicated, and resolving the
//! frontend-supplied id through the same traversal guard every other
//! library entry point uses.

use super::guard;
use std::path::Path;

/// Reveals bundle `id` in Finder, selecting its replay file. `id` is
/// untrusted frontend input, so it clears `resolve_replay_file` before any
/// path is touched; a bundle that is not on disk fails with the same
/// `library_replay_missing` code `open_replay_file` reports rather than
/// handing a nonexistent path to Finder.
pub(crate) fn reveal_bundle(destination: &Path, id: &str) -> Result<(), String> {
    let file = guard::resolve_replay_file(destination, id)?;
    if !file.is_file() {
        return Err("library_replay_missing".to_string());
    }
    crate::replay::reveal_in_finder(&file).map_err(|_| "library_reveal_failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guard has to reject before anything reaches Finder, so these
    /// ids must fail without the destination existing at all.
    #[test]
    fn rejects_ids_that_could_escape_the_destination() {
        let destination = Path::new("/tmp/encore-exports-does-not-exist");

        for id in ["..", ".", "", "../escape", "a/b", "a\\b", "/etc/passwd"] {
            assert_eq!(
                reveal_bundle(destination, id),
                Err("library_invalid_id".to_string()),
                "expected {id:?} to be rejected"
            );
        }
    }

    #[test]
    fn reports_a_missing_bundle_instead_of_revealing_nothing() {
        let destination = std::env::temp_dir().join("encore-reveal-no-such-destination");

        assert_eq!(
            reveal_bundle(&destination, "Encore Replay 2026-08-14 10.00.00"),
            Err("library_replay_missing".to_string())
        );
    }
}
