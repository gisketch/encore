use super::super::{
    BundleMediaDescription, BundleStreamDescription, FfmpegRunner, PackageFileSystem,
    RunnerFailure, SystemPackageFileSystem,
};
use crate::{
    capture::{
        CaptureGeometry, CaptureState, EvidenceCaptureFacts, PipelineState, SegmentBoundary,
        SegmentRecord, SourceKind,
    },
    retention::RollingStore,
};
use std::{
    collections::VecDeque,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

pub(crate) struct TestDirectory(PathBuf);

impl TestDirectory {
    pub(crate) fn new(label: &str) -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "encore-packager-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) struct FakeRunner {
    pub(super) inspections: Mutex<VecDeque<Result<BundleMediaDescription, RunnerFailure>>>,
    pub(super) concat_result: Result<(), RunnerFailure>,
    pub(super) manifest: Mutex<Option<String>>,
}

impl FakeRunner {
    pub(super) fn compatible(segment_count: usize) -> Self {
        Self {
            inspections: Mutex::new(
                std::iter::repeat_with(|| Ok(h264_media()))
                    .take(segment_count)
                    .collect(),
            ),
            concat_result: Ok(()),
            manifest: Mutex::new(None),
        }
    }

    pub(super) fn failing_inspection(failure: RunnerFailure) -> Self {
        Self {
            inspections: Mutex::new(VecDeque::from([Err(failure)])),
            concat_result: Ok(()),
            manifest: Mutex::new(None),
        }
    }
}

impl FfmpegRunner for FakeRunner {
    fn inspect(&self, _input: &Path) -> Result<BundleMediaDescription, RunnerFailure> {
        self.inspections.lock().unwrap().pop_front().unwrap()
    }

    fn concat(&self, manifest: &Path, output: &Path) -> Result<(), RunnerFailure> {
        *self.manifest.lock().unwrap() = Some(fs::read_to_string(manifest).unwrap());
        self.concat_result?;
        fs::write(output, b"golden-h264-mp4").unwrap();
        Ok(())
    }
}

pub(super) fn h264_media() -> BundleMediaDescription {
    BundleMediaDescription {
        streams: vec![BundleStreamDescription {
            kind: "video".into(),
            codec: "h264".into(),
            width: Some(1920),
            height: Some(1080),
            pixel_format: Some("yuv420p".into()),
        }],
    }
}

pub(super) fn store_with_segments(directory: &TestDirectory, source_ids: &[&str]) -> RollingStore {
    let store = RollingStore::open(directory.path().to_owned(), 10).unwrap();
    for (index, source_id) in source_ids.iter().enumerate() {
        let time = 10_000 + u64::try_from(index).unwrap() * 10_000;
        let path = directory
            .path()
            .join(format!("segment-{time}-{index:06}.mp4"));
        fs::write(&path, b"segment bytes").unwrap();
        store
            .admit(&SegmentRecord {
                path: path.to_string_lossy().into_owned(),
                source_id: (*source_id).into(),
                width: 1920,
                height: 1080,
                byte_size: 13,
                duration_micros: 10_000_000,
                session_start_micros: time * 1_000,
                session_end_micros: (time + 10_000) * 1_000,
                completed_at_unix_ms: time + 10_000,
                boundary: SegmentBoundary::Duration,
            })
            .unwrap();
    }
    store
}

pub(crate) fn capture_facts() -> EvidenceCaptureFacts {
    EvidenceCaptureFacts {
        source_kind: Some(SourceKind::Display),
        geometry: Some(CaptureGeometry {
            width: 1920,
            height: 1080,
        }),
        capture_state: CaptureState::Capturing,
        encoder_state: PipelineState::Encoding,
        retention_minutes: 10,
        dropped_frames: 3,
        evidence_gap_count: 2,
        retry_count: 1,
    }
}

pub(super) struct FailingMetadataFileSystem;

impl PackageFileSystem for FailingMetadataFileSystem {
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        SystemPackageFileSystem.create_dir_all(path)
    }
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        SystemPackageFileSystem.create_dir(path)
    }
    fn create_lock(&self, path: &Path) -> io::Result<()> {
        SystemPackageFileSystem.create_lock(path)
    }
    fn try_exists(&self, path: &Path) -> io::Result<bool> {
        SystemPackageFileSystem.try_exists(path)
    }
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        SystemPackageFileSystem.read_dir(path)
    }
    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        if path.file_name().is_some_and(|name| name == "metadata.json") {
            return Err(io::Error::other("injected metadata failure"));
        }
        SystemPackageFileSystem.write_synced(path, bytes)
    }
    fn file_len(&self, path: &Path) -> io::Result<u64> {
        SystemPackageFileSystem.file_len(path)
    }
    fn sync_file(&self, path: &Path) -> io::Result<()> {
        SystemPackageFileSystem.sync_file(path)
    }
    fn sync_dir(&self, path: &Path) -> io::Result<()> {
        SystemPackageFileSystem.sync_dir(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        SystemPackageFileSystem.rename(from, to)
    }
    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        SystemPackageFileSystem.remove_dir_all(path)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        SystemPackageFileSystem.remove_file(path)
    }
}
