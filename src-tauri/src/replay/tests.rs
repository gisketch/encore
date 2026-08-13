use super::{
    ReplayClock, ReplayService, ReplayState, ReplayTime, ShortcutRegistrationSnapshot,
    ShortcutRegistrationState,
};
use crate::{
    capture::{SegmentBoundary, SegmentRecord},
    packager::{
        BundleMediaDescription, BundleStreamDescription, FfmpegRunner, ReplayPackager,
        RunnerFailure,
    },
    retention::RollingStore,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier, Mutex,
    },
    thread,
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

#[path = "tests/destination.rs"]
mod destination;
#[path = "tests/lifecycle.rs"]
mod lifecycle;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("encore-replay-test-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn segment(&self, time_ms: u64, sequence: u64, byte_size: u64) -> SegmentRecord {
        let path = self.0.join(format!("segment-{time_ms}-{sequence:06}.mp4"));
        fs::write(&path, vec![1_u8; byte_size.try_into().unwrap()]).unwrap();
        SegmentRecord {
            path: path.to_string_lossy().into_owned(),
            source_id: "display:1".into(),
            width: 1920,
            height: 1080,
            byte_size,
            duration_micros: 10_000_000,
            session_start_micros: time_ms * 1_000,
            session_end_micros: (time_ms + 10_000) * 1_000,
            completed_at_unix_ms: time_ms + 10_000,
            boundary: SegmentBoundary::Duration,
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct TestClock(Mutex<Result<ReplayTime, ()>>);

impl TestClock {
    fn new(now: u64) -> Self {
        Self(Mutex::new(Ok(ReplayTime { unix_ms: now })))
    }

    fn set(&self, now: u64) {
        *self.0.lock().unwrap() = Ok(ReplayTime { unix_ms: now });
    }

    fn fail(&self) {
        *self.0.lock().unwrap() = Err(());
    }
}

impl ReplayClock for TestClock {
    fn now(&self) -> Result<ReplayTime, ()> {
        *self.0.lock().unwrap()
    }
}

fn store_with_segments(times: &[(u64, u64)]) -> (TestDirectory, RollingStore) {
    let directory = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 10).unwrap();
    for (sequence, (time_ms, byte_size)) in times.iter().copied().enumerate() {
        let record = directory.segment(time_ms, sequence.try_into().unwrap(), byte_size);
        store.admit(&record).unwrap();
    }
    (directory, store)
}

struct ConfigurableRunner {
    concat_result: Mutex<Result<(), RunnerFailure>>,
}

impl ConfigurableRunner {
    fn new(concat_result: Result<(), RunnerFailure>) -> Self {
        Self {
            concat_result: Mutex::new(concat_result),
        }
    }

    fn set_concat_result(&self, result: Result<(), RunnerFailure>) {
        *self.concat_result.lock().unwrap() = result;
    }
}

impl FfmpegRunner for ConfigurableRunner {
    fn inspect(&self, _input: &Path) -> Result<BundleMediaDescription, RunnerFailure> {
        Ok(BundleMediaDescription {
            streams: vec![BundleStreamDescription {
                kind: "video".into(),
                codec: "h264".into(),
                width: Some(1920),
                height: Some(1080),
                pixel_format: Some("yuv420p".into()),
            }],
        })
    }

    fn concat(&self, _manifest: &Path, output: &Path) -> Result<(), RunnerFailure> {
        (*self.concat_result.lock().unwrap())?;
        fs::write(output, b"deterministic replay").map_err(|_| RunnerFailure::Failed)
    }
}

fn service_with_clock(rolling: RollingStore, clock: Arc<dyn ReplayClock>) -> ReplayService {
    let destination = std::env::temp_dir().join(format!(
        "encore-replay-service-test-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    ReplayService::with_test_parts(
        rolling,
        clock,
        Arc::new(ReplayPackager::new(Arc::new(ConfigurableRunner::new(Ok(
            (),
        ))))),
        destination::fixed(destination),
        Arc::new(|_| Ok(())),
    )
}

#[test]
fn empty_retention_returns_the_stable_unavailable_code() {
    let service = service_with_clock(
        RollingStore::empty_for_tests(),
        Arc::new(TestClock::new(1_000)),
    );

    assert_eq!(service.trigger(), Err("replay_unavailable".into()));
    assert_eq!(service.snapshot().state, ReplayState::Unavailable);
    assert_eq!(
        service.snapshot().error_code.as_deref(),
        Some("replay_unavailable")
    );
}

#[test]
fn shortcut_registration_failure_is_typed_without_blocking_manual_replay() {
    let (_directory, store) = store_with_segments(&[(10_000, 16)]);
    let service = service_with_clock(store, Arc::new(TestClock::new(1_000)));

    service.set_shortcut_registration(ShortcutRegistrationSnapshot {
        state: ShortcutRegistrationState::Unavailable,
        error_code: Some("shortcut_registration_failed".into()),
    });
    let unavailable = service.snapshot();
    assert_eq!(
        unavailable.shortcut.state,
        ShortcutRegistrationState::Unavailable
    );
    assert_eq!(
        unavailable.shortcut.error_code.as_deref(),
        Some("shortcut_registration_failed")
    );

    let replay = service.trigger().unwrap();
    assert_eq!(replay.state, ReplayState::Preparing);
    assert_eq!(
        replay.shortcut.error_code.as_deref(),
        Some("shortcut_registration_failed")
    );

    service.set_shortcut_registration(ShortcutRegistrationSnapshot {
        state: ShortcutRegistrationState::Registered,
        error_code: None,
    });
    let registered = service.snapshot();
    assert_eq!(
        registered.shortcut.state,
        ShortcutRegistrationState::Registered
    );
    assert_eq!(registered.shortcut.error_code, None);
}

#[test]
fn trigger_creates_an_ordered_privacy_safe_snapshot() {
    let (_directory, store) = store_with_segments(&[(30_000, 4), (10_000, 16), (20_000, 8)]);
    let lease = store.lease().unwrap();
    assert_eq!(
        lease
            .segments()
            .iter()
            .map(|segment| segment.evidence_start_unix_ms())
            .collect::<Vec<_>>(),
        vec![10_000, 20_000, 30_000]
    );
    drop(lease);
    let service = service_with_clock(store, Arc::new(TestClock::new(1_000)));

    let replay = service.trigger().unwrap();
    let pending = replay.pending.clone().unwrap();

    assert_eq!(replay.state, ReplayState::Preparing);
    assert_eq!(pending.id, "replay-1");
    assert_eq!(pending.created_at_unix_ms, 1_000);
    assert_eq!(pending.segment_count, 3);
    assert_eq!(pending.total_bytes, 28);
    assert_eq!(pending.evidence_start_unix_ms, 10_000);
    assert_eq!(pending.evidence_end_unix_ms, 40_000);
    let serialized = serde_json::to_string(&replay).unwrap();
    assert!(!serialized.contains("segment-"));
    assert!(!serialized.contains("display:1"));
    assert!(!serialized.contains("encore-replay-test"));
}

#[test]
fn triggers_coalesce_while_an_export_is_active() {
    let (_directory, store) = store_with_segments(&[(10_000, 16)]);
    let clock = Arc::new(TestClock::new(1_000));
    let service = service_with_clock(store, clock.clone());

    let first = service.trigger().unwrap();
    let coalesced = service.trigger().unwrap();

    assert_eq!(first.pending.as_ref().unwrap().id, "replay-1");
    assert_eq!(coalesced.pending.as_ref().unwrap().id, "replay-1");
    assert_eq!(coalesced.state, ReplayState::Preparing);
}

#[test]
fn concurrent_triggers_serialize_to_one_pending_replay() {
    let (_directory, store) = store_with_segments(&[(10_000, 16)]);
    let service = service_with_clock(store, Arc::new(TestClock::new(1_000)));
    let callers = 8;
    let barrier = Arc::new(Barrier::new(callers));
    let mut workers = Vec::new();

    for _ in 0..callers {
        let service = service.clone();
        let barrier = barrier.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            service.trigger().unwrap().pending.unwrap().id
        }));
    }

    for worker in workers {
        assert_eq!(worker.join().unwrap(), "replay-1");
    }
}
