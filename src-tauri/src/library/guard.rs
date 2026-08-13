use std::path::{Path, PathBuf};

/// Resolves a library entry id (a bundle folder name, as returned by
/// `scan`) to its bundle folder inside `destination`. `id` always comes
/// from the frontend, so this is the one seam in the library that has to
/// distrust it: any path separator, `.`/`..` segment, or empty string is
/// rejected before it ever reaches a join, and the joined path is checked
/// to still sit directly inside `destination` as a defense-in-depth
/// second gate. Both `open_replay_file` and `delete_replay` build on this
/// single check.
pub(crate) fn resolve_bundle_dir(destination: &Path, id: &str) -> Result<PathBuf, String> {
    let is_safe_component =
        !id.is_empty() && id != "." && id != ".." && !id.contains('/') && !id.contains('\\');
    if !is_safe_component {
        return Err("library_invalid_id".to_string());
    }
    let bundle = destination.join(id);
    if bundle.parent() != Some(destination) {
        return Err("library_invalid_id".to_string());
    }
    Ok(bundle)
}

/// Resolves a library entry id to its replay file inside `destination`,
/// via `resolve_bundle_dir`.
pub(crate) fn resolve_replay_file(destination: &Path, id: &str) -> Result<PathBuf, String> {
    resolve_bundle_dir(destination, id).map(|bundle| bundle.join(crate::packager::REPLAY_FILENAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_plain_id_inside_the_destination() {
        let destination = Path::new("/tmp/encore-exports");

        let resolved =
            resolve_replay_file(destination, "Encore Replay 2026-08-13 09.00.00").unwrap();

        assert_eq!(
            resolved,
            destination
                .join("Encore Replay 2026-08-13 09.00.00")
                .join("replay.mp4")
        );
    }

    #[test]
    fn rejects_ids_that_could_escape_the_destination() {
        let destination = Path::new("/tmp/encore-exports");

        for id in ["..", ".", "", "../escape", "a/b", "a\\b", "/etc/passwd"] {
            assert!(
                resolve_replay_file(destination, id).is_err(),
                "expected {id:?} to be rejected"
            );
        }
    }
}
