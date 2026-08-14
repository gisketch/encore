use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path};

/// Everything the post-save preview needs about one saved replay. Built
/// from the bundle on disk rather than from anything the frontend hands
/// back, and degraded rather than invented: `duration_seconds` is `None`
/// whenever `metadata.json` is missing, unparseable, or does not record an
/// evidence window, so the preview omits that segment instead of guessing.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewPayload {
    pub id: String,
    /// The same "Today, 4:32 PM" title convention the Editor header uses —
    /// literally the Editor header's own value, so the two windows can
    /// never drift apart on how a replay is named.
    pub display_name: String,
    pub duration_seconds: Option<u64>,
    pub total_bytes: u64,
    /// Absolute filesystem path to `replay.mp4`, for the frontend to hand
    /// to `convertFileSrc`.
    pub video_path: String,
}

/// Builds the preview payload for bundle `id` inside `destination`.
///
/// `id` is untrusted frontend input, so it goes through
/// `library::resolve_bundle_dir` — the one traversal guard the Library and
/// Editor already share — which rejects anything that could escape
/// `destination` with the stable `library_invalid_id` code. Name, size,
/// and video path are read through `editor::header`, so this adds no
/// second opinion about how a bundle is described; only the duration,
/// which the header has no use for, is read here.
pub(crate) fn build(destination: &Path, id: &str) -> Result<PreviewPayload, String> {
    let bundle = crate::library::resolve_bundle_dir(destination, id)?;
    let header = crate::editor::header(destination, id)?;
    Ok(PreviewPayload {
        id: header.id,
        display_name: header.title,
        duration_seconds: duration_seconds(&bundle),
        total_bytes: header.total_bytes,
        video_path: header.video_path,
    })
}

/// The evidence bundle schema (`packager::model::metadata_json`) records an
/// evidence window, not a duration field — this derives seconds from it,
/// mirroring `library::scan`'s reading rather than raising that committed
/// file's complexity by exporting from it.
fn duration_seconds(bundle_directory: &Path) -> Option<u64> {
    let bytes = fs::read(bundle_directory.join(crate::packager::METADATA_FILENAME)).ok()?;
    let metadata: Value = serde_json::from_slice(&bytes).ok()?;
    let start = metadata["evidence"]["startUnixMs"].as_u64()?;
    let end = metadata["evidence"]["endUnixMs"].as_u64()?;
    end.checked_sub(start).map(|elapsed_ms| elapsed_ms / 1000)
}

#[cfg(test)]
mod tests;
