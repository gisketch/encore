use super::*;
use std::sync::mpsc;

#[test]
fn successful_export_releases_its_lease_before_a_later_trigger() {
    let directory = TestDirectory::new();
    let destination = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 5).unwrap();
    for sequence in 0..=2 {
        store
            .admit(&directory.segment(sequence * 10_000, sequence, 16))
            .unwrap();
    }
    let clock = Arc::new(TestClock::new(1_000));
    let service = ReplayService::with_test_parts(
        store.clone(),
        clock.clone(),
        Arc::new(ReplayPackager::new(Arc::new(ConfigurableRunner::new(Ok(
            (),
        ))))),
        destination::fixed(destination.path().to_owned()),
        Arc::new(|_| Ok(())),
    );
    let first_id = service.trigger_for_export().unwrap().replay_id.unwrap();
    assert_eq!(service.run_export(&first_id).state, ReplayState::Saved);
    let first_path = directory.path().join("segment-0-000000.mp4");

    for sequence in 3..=40 {
        store
            .admit(&directory.segment(sequence * 10_000, sequence, 16))
            .unwrap();
    }
    assert!(!first_path.exists());
    clock.set(2_000);
    assert_eq!(
        service.trigger().unwrap().pending.unwrap().segment_count,
        30
    );
}

#[test]
fn failed_export_preserves_the_lease_and_retry_releases_it() {
    let directory = TestDirectory::new();
    let destination = TestDirectory::new();
    let store = RollingStore::open(directory.path().to_owned(), 5).unwrap();
    for sequence in 0..=2 {
        store
            .admit(&directory.segment(sequence * 10_000, sequence, 16))
            .unwrap();
    }
    let runner = Arc::new(ConfigurableRunner::new(Err(RunnerFailure::Failed)));
    let service = ReplayService::with_test_parts(
        store.clone(),
        Arc::new(TestClock::new(1_000)),
        Arc::new(ReplayPackager::new(runner.clone())),
        destination::fixed(destination.path().to_owned()),
        Arc::new(|_| Ok(())),
    );
    let first_id = service.trigger_for_export().unwrap().replay_id.unwrap();
    let failed = service.run_export(&first_id);

    assert_eq!(failed.state, ReplayState::Failed);
    assert_eq!(failed.error_code.as_deref(), Some("export_concat_failed"));
    assert_eq!(failed.pending.unwrap().id, first_id);
    let first_path = directory.path().join("segment-0-000000.mp4");
    for sequence in 3..=40 {
        store
            .admit(&directory.segment(sequence * 10_000, sequence, 16))
            .unwrap();
    }
    assert!(first_path.exists());

    runner.set_concat_result(Ok(()));
    let retry = service.retry_for_export().unwrap();
    assert_eq!(retry.snapshot.state, ReplayState::Preparing);
    assert_eq!(retry.replay_id.as_deref(), Some(first_id.as_str()));
    let saved = service.run_export(&first_id);
    assert_eq!(saved.state, ReplayState::Saved);
    assert_eq!(saved.pending, None);
    store.prune().unwrap();
    assert!(!first_path.exists());
}

#[test]
fn internal_clock_failures_are_stable_and_preserve_pending_evidence() {
    let (_directory, store) = store_with_segments(&[(10_000, 16)]);
    let destination = TestDirectory::new();
    let clock = Arc::new(TestClock::new(1_000));
    let service = ReplayService::with_test_parts(
        store,
        clock.clone(),
        Arc::new(ReplayPackager::new(Arc::new(ConfigurableRunner::new(Err(
            RunnerFailure::Failed,
        ))))),
        destination::fixed(destination.path().to_owned()),
        Arc::new(|_| Ok(())),
    );
    let first = service.trigger_for_export().unwrap().replay_id.unwrap();
    service.run_export(&first);
    clock.fail();

    assert_eq!(service.trigger(), Err("replay_trigger_failed".into()));
    let snapshot = service.snapshot();
    assert_eq!(snapshot.pending.unwrap().id, first);
    assert_eq!(
        snapshot.error_code.as_deref(),
        Some("replay_trigger_failed")
    );
}

#[test]
fn finder_reveal_uses_an_opaque_saved_id_without_emitting_paths() {
    let (directory, store) = store_with_segments(&[(10_000, 16)]);
    let destination = TestDirectory::new();
    let (revealed, receiver) = mpsc::channel();
    let service = ReplayService::with_test_parts(
        store,
        Arc::new(TestClock::new(1_000)),
        Arc::new(ReplayPackager::new(Arc::new(ConfigurableRunner::new(Ok(
            (),
        ))))),
        destination::fixed(destination.path().to_owned()),
        Arc::new(move |path| {
            revealed.send(path.to_owned()).unwrap();
            Ok(())
        }),
    );
    let id = service.trigger_for_export().unwrap().replay_id.unwrap();
    let saved = service.run_export(&id);

    assert_eq!(saved.state, ReplayState::Saved);
    assert_eq!(
        saved.saved.as_ref().map(|saved| saved.id.as_str()),
        Some(id.as_str())
    );
    assert!(serde_json::to_string(&saved)
        .unwrap()
        .contains("displayName"));
    let serialized = serde_json::to_string(&saved).unwrap();
    assert!(!serialized.contains("segment-"));
    assert!(!serialized.contains(directory.path().to_string_lossy().as_ref()));
    assert!(!serialized.contains(destination.path().to_string_lossy().as_ref()));

    assert_eq!(service.reveal_saved(&id).unwrap().state, ReplayState::Saved);
    assert_eq!(
        receiver.recv().unwrap().file_name().unwrap(),
        std::ffi::OsStr::new("replay.mp4")
    );
    assert_eq!(
        service.reveal_saved("caller-provided-path"),
        Err("export_reveal_failed".into())
    );
}

/// The destination lookup is re-read for every export rather than fixed at
/// construction: changing the shared value between two saves (mirroring
/// what Settings does through `CaptureService::set_save_destination`) must
/// land the second save in the new folder without touching the first.
#[test]
fn destination_lookup_is_read_fresh_for_every_export() {
    let (_directory, store) = store_with_segments(&[(10_000, 16), (20_000, 16), (30_000, 16)]);
    let first_destination = TestDirectory::new();
    let second_destination = TestDirectory::new();
    let current = destination::Swappable::new(first_destination.path().to_owned());
    let service = ReplayService::with_test_parts(
        store,
        Arc::new(TestClock::new(1_000)),
        Arc::new(ReplayPackager::new(Arc::new(ConfigurableRunner::new(Ok(
            (),
        ))))),
        current.lookup(),
        Arc::new(|_| Ok(())),
    );

    let first_id = service.trigger_for_export().unwrap().replay_id.unwrap();
    let first_saved = service.run_export(&first_id);
    assert_eq!(first_saved.state, ReplayState::Saved);
    assert_eq!(fs::read_dir(first_destination.path()).unwrap().count(), 1);

    current.set(second_destination.path().to_owned());
    let second_id = service.trigger_for_export().unwrap().replay_id.unwrap();
    let second_saved = service.run_export(&second_id);
    assert_eq!(second_saved.state, ReplayState::Saved);
    assert_eq!(fs::read_dir(first_destination.path()).unwrap().count(), 1);
    assert_eq!(fs::read_dir(second_destination.path()).unwrap().count(), 1);
}

/// A destination that cannot be created at save time (here, a path nested
/// under a plain file) fails the save through the same actionable error
/// code the folder-picker validation uses, rather than failing silently or
/// panicking.
#[test]
fn unusable_destination_fails_the_save_with_the_actionable_error_code() {
    let (_directory, store) = store_with_segments(&[(10_000, 16)]);
    let blocking_file = TestDirectory::new();
    let not_a_directory = blocking_file.path().join("not-a-directory");
    fs::write(&not_a_directory, b"not a directory").unwrap();
    let unusable = not_a_directory.join("bundles");
    let service = ReplayService::with_test_parts(
        store,
        Arc::new(TestClock::new(1_000)),
        Arc::new(ReplayPackager::new(Arc::new(ConfigurableRunner::new(Ok(
            (),
        ))))),
        destination::fixed(unusable),
        Arc::new(|_| Ok(())),
    );

    let id = service.trigger_for_export().unwrap().replay_id.unwrap();
    let result = service.run_export(&id);

    assert_eq!(result.state, ReplayState::Failed);
    assert_eq!(
        result.error_code.as_deref(),
        Some("export_destination_unavailable")
    );
}
