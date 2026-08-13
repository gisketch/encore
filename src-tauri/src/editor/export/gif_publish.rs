//! Publishing a built GIF as a plain sibling FILE (not a bundle directory)
//! in the destination — the ticket's "a plain .gif file in the destination
//! is fine" decision. A new file (mirroring `publish.rs`'s shape for the
//! MP4 bundle path) rather than an addition to that already-tracked file,
//! per the harness's zero-tolerance for complexity increases there.

use crate::packager::PackageFileSystem;
use std::path::{Path, PathBuf};

const MAX_NAME_SUFFIX: u32 = 10_000;

/// Publishes a built GIF as a sibling FILE in `destination`, named
/// `"{source_name} (trimmed).gif"` (deduped with " 2.gif", " 3.gif", ...
/// on collision). Unlike `publish::publish_trimmed`, there is no bundle
/// directory or lock dance: the GIF is already fully built at `built_gif`
/// by the time this runs, so a plain check-then-rename is enough to avoid
/// clobbering an existing file, and a rename racing another export at most
/// retries the next suffix.
pub(crate) fn publish_gif(
    files: &dyn PackageFileSystem,
    destination: &Path,
    source_name: &str,
    built_gif: &Path,
) -> Result<PathBuf, String> {
    files
        .create_dir_all(destination)
        .map_err(|_| "export_destination_unavailable".to_string())?;

    for suffix in 0..=MAX_NAME_SUFFIX {
        let name = trimmed_gif_display_name(source_name, suffix);
        let target = destination.join(&name);
        if exists(files, &target)? {
            continue;
        }
        match files.rename(built_gif, &target) {
            Ok(()) => {
                files
                    .sync_dir(destination)
                    .map_err(|_| "export_destination_unavailable".to_string())?;
                return Ok(target);
            }
            Err(_) if files.try_exists(&target).unwrap_or(false) => continue,
            Err(_) => return Err("export_destination_unavailable".into()),
        }
    }
    Err("export_destination_unavailable".into())
}

fn exists(files: &dyn PackageFileSystem, path: &Path) -> Result<bool, String> {
    files
        .try_exists(path)
        .map_err(|_| "export_destination_unavailable".to_string())
}

fn trimmed_gif_display_name(source_name: &str, suffix: u32) -> String {
    let suffix_text = if suffix == 0 {
        String::new()
    } else {
        format!(" {suffix}")
    };
    format!("{source_name} (trimmed){suffix_text}.gif")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packager::SystemPackageFileSystem;
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
                "encore-export-gif-publish-{label}-{}-{id}",
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
    fn suffix_zero_has_no_trailing_number() {
        assert_eq!(
            trimmed_gif_display_name("Encore Replay X", 0),
            "Encore Replay X (trimmed).gif"
        );
    }

    #[test]
    fn later_suffixes_dedupe_before_the_extension() {
        assert_eq!(
            trimmed_gif_display_name("Encore Replay X", 2),
            "Encore Replay X (trimmed) 2.gif"
        );
    }

    #[test]
    fn writes_the_built_file_at_the_deduped_target() {
        let root = TestDirectory::new("publish");
        let files = SystemPackageFileSystem;
        let built = root.path().join("scratch.gif");
        fs::write(&built, b"GIF89a-fake").unwrap();

        let published = publish_gif(&files, root.path(), "Encore Replay X", &built).unwrap();

        assert_eq!(
            published.file_name().unwrap().to_string_lossy(),
            "Encore Replay X (trimmed).gif"
        );
        assert_eq!(fs::read(&published).unwrap(), b"GIF89a-fake");
        assert!(!built.exists()); // renamed away, not copied
    }

    #[test]
    fn dedupes_a_second_export_of_the_same_source() {
        let root = TestDirectory::new("collision");
        let files = SystemPackageFileSystem;
        let first_built = root.path().join("first.gif");
        fs::write(&first_built, b"first").unwrap();
        let second_built = root.path().join("second.gif");
        fs::write(&second_built, b"second").unwrap();

        let first = publish_gif(&files, root.path(), "Encore Replay Y", &first_built).unwrap();
        let second = publish_gif(&files, root.path(), "Encore Replay Y", &second_built).unwrap();

        assert_eq!(
            first.file_name().unwrap().to_string_lossy(),
            "Encore Replay Y (trimmed).gif"
        );
        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            "Encore Replay Y (trimmed) 1.gif"
        );
    }
}
