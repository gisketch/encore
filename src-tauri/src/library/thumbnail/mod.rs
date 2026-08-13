use super::guard;
#[cfg(test)]
mod tests;

use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};

const THUMBNAIL_SUBDIR: &str = "thumbnails";
const JPEG_SUFFIX: &str = ".jpg";
const FAILED_SUFFIX: &str = ".failed";
/// `ffmpeg -ss 1 -i replay.mp4 -frames:v 1 -vf scale=320:-2 -q:v 5`, per the
/// spec's grilled decision: first keyframe at least one second in, scaled
/// to a 320px-wide JPEG.
const SEEK_SECONDS: &str = "1";
const SCALE_FILTER: &str = "scale=320:-2";
const JPEG_QUALITY: &str = "5";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThumbnailFailure {
    Unavailable,
}

/// Extracts one JPEG frame from `input` into `output`. A trait so tests can
/// inject a fake instead of shelling out to the real ffmpeg sidecar.
pub(crate) trait ThumbnailExtractor: Send + Sync {
    fn extract(&self, input: &Path, output: &Path) -> Result<(), ThumbnailFailure>;
}

/// Real extractor: shells out to the bundled ffmpeg sidecar, mirroring
/// `packager::runner::ProcessFfmpegRunner`'s invocation shape.
pub(crate) struct ProcessThumbnailExtractor {
    ffmpeg: PathBuf,
}

impl ProcessThumbnailExtractor {
    pub(crate) fn new(ffmpeg: PathBuf) -> Self {
        Self { ffmpeg }
    }
}

impl ThumbnailExtractor for ProcessThumbnailExtractor {
    fn extract(&self, input: &Path, output: &Path) -> Result<(), ThumbnailFailure> {
        let status = Command::new(&self.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                SEEK_SECONDS,
                "-i",
            ])
            .arg(input)
            .args(["-frames:v", "1", "-vf", SCALE_FILTER, "-q:v", JPEG_QUALITY])
            .arg(output)
            .status()
            .map_err(|_| ThumbnailFailure::Unavailable)?;
        status
            .success()
            .then_some(())
            .ok_or(ThumbnailFailure::Unavailable)
    }
}

/// Cache key for one replay's thumbnail: the replay file's path, byte
/// size, and mtime, hashed into a filename-safe string. Replacing or
/// re-saving the same bundle path changes the size and/or mtime, which
/// changes the key, which is exactly what "regenerate lazily only when the
/// key changes" needs — no explicit invalidation bookkeeping required.
/// `DefaultHasher` (not `HashMap`'s randomized `RandomState`) is
/// deterministic within one compiled binary, which is all a same-run,
/// same-toolchain cache needs; a toolchain upgrade shuffling keys only
/// costs a harmless regeneration.
fn cache_key(replay_file: &Path, size: u64, modified_unix_ms: u64) -> String {
    let mut hasher = DefaultHasher::new();
    replay_file.hash(&mut hasher);
    size.hash(&mut hasher);
    modified_unix_ms.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn modified_unix_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|since_epoch| u64::try_from(since_epoch.as_millis()).ok())
        .unwrap_or(0)
}

fn jpeg_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_dir
        .join(THUMBNAIL_SUBDIR)
        .join(format!("{key}{JPEG_SUFFIX}"))
}

fn failed_marker_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_dir
        .join(THUMBNAIL_SUBDIR)
        .join(format!("{key}{FAILED_SUFFIX}"))
}

/// Returns the cached JPEG bytes for `id`'s replay file inside
/// `destination`, generating it into `cache_dir/thumbnails/` on a cache
/// miss via `extractor` — the export folder is never written to. A prior
/// failed extraction for this exact key short-circuits to an error without
/// touching `extractor` again, so a broken video never retries in a loop;
/// the caller (the `library_thumbnail` command) turns any error into a
/// permanent placeholder for the frontend.
pub(crate) fn thumbnail_bytes(
    destination: &Path,
    cache_dir: &Path,
    id: &str,
    extractor: &dyn ThumbnailExtractor,
) -> Result<Vec<u8>, String> {
    let replay_file = guard::resolve_replay_file(destination, id)?;
    let metadata =
        fs::metadata(&replay_file).map_err(|_| "library_thumbnail_missing".to_string())?;
    let key = cache_key(&replay_file, metadata.len(), modified_unix_ms(&metadata));
    let jpeg_path = jpeg_path(cache_dir, &key);
    let failed_path = failed_marker_path(cache_dir, &key);

    if let Ok(bytes) = fs::read(&jpeg_path) {
        return Ok(bytes);
    }
    if failed_path.is_file() {
        return Err("library_thumbnail_unavailable".to_string());
    }

    fs::create_dir_all(cache_dir.join(THUMBNAIL_SUBDIR))
        .map_err(|_| "library_thumbnail_unavailable".to_string())?;

    if extractor.extract(&replay_file, &jpeg_path).is_ok() {
        if let Ok(bytes) = fs::read(&jpeg_path) {
            return Ok(bytes);
        }
    }
    let _ = fs::write(&failed_path, b"");
    Err("library_thumbnail_unavailable".to_string())
}
