use super::super::{ExportRunner, KeepSegment};
use crate::{editor::keyframes::KeyframeProbe, packager::RunnerFailure};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

pub(super) struct TestDirectory(PathBuf);

impl TestDirectory {
    pub(super) fn new(label: &str) -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "encore-editor-export-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Writes a minimal source bundle (`replay.mp4` + `metadata.json`) at
/// `root/name`, the same shape `packager::package` produces.
pub(super) fn write_source_bundle(root: &Path, name: &str, video_bytes: &[u8]) -> PathBuf {
    let bundle = root.join(name);
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("replay.mp4"), video_bytes).unwrap();
    fs::write(
        bundle.join("metadata.json"),
        r#"{"schemaVersion":1,"replayId":"replay-source","createdAtUnixMs":1000}"#,
    )
    .unwrap();
    bundle
}

pub(super) fn keep(start: f64, end: f64) -> KeepSegment {
    KeepSegment { start, end }
}

/// A canned keyframe list (0, 1, 2, 3 seconds, 3.5s clip) parsed the same
/// way `editor::keyframes::probe` parses real ffprobe CSV output.
pub(super) struct FakeProbe;

impl KeyframeProbe for FakeProbe {
    fn packet_csv(&self, _input: &Path) -> Result<String, crate::editor::keyframes::ProbeFailure> {
        Ok("0.000000,K__\n1.000000,K__\n2.000000,K__\n3.000000,K__\n".to_string())
    }

    fn duration_csv(
        &self,
        _input: &Path,
    ) -> Result<String, crate::editor::keyframes::ProbeFailure> {
        Ok("3.500000\n".to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TrimCall {
    pub start: f64,
    pub end: f64,
}

/// Records every `trim`/`concat` call's arguments (for exact-sequence
/// assertions) and writes a placeholder file at each requested output
/// path, standing in for what a real ffmpeg stream-copy would produce.
pub(super) struct FakeExportRunner {
    pub trims: Mutex<Vec<TrimCall>>,
    pub concat_calls: Mutex<u32>,
    pub last_manifest: Mutex<Option<String>>,
    pub fail_trim: bool,
}

impl FakeExportRunner {
    pub(super) fn new() -> Self {
        Self {
            trims: Mutex::new(Vec::new()),
            concat_calls: Mutex::new(0),
            last_manifest: Mutex::new(None),
            fail_trim: false,
        }
    }

    pub(super) fn failing() -> Self {
        Self {
            fail_trim: true,
            ..Self::new()
        }
    }
}

impl ExportRunner for FakeExportRunner {
    fn trim(
        &self,
        _input: &Path,
        start: f64,
        end: f64,
        output: &Path,
    ) -> Result<(), RunnerFailure> {
        if self.fail_trim {
            return Err(RunnerFailure::Failed);
        }
        self.trims.lock().unwrap().push(TrimCall { start, end });
        fs::write(output, b"trimmed-segment").unwrap();
        Ok(())
    }

    fn concat(&self, manifest: &Path, output: &Path) -> Result<(), RunnerFailure> {
        *self.concat_calls.lock().unwrap() += 1;
        *self.last_manifest.lock().unwrap() = Some(fs::read_to_string(manifest).unwrap());
        fs::write(output, b"concatenated-replay").unwrap();
        Ok(())
    }
}
