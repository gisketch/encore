use super::*;
use crate::diagnostics::{DiagnosticDomain, DiagnosticEntry, DiagnosticLog};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_LOG_PATH: AtomicU64 = AtomicU64::new(1);

fn scratch_log_path() -> std::path::PathBuf {
    let id = NEXT_LOG_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "encore-capture-diagnostics-test-{}-{id}.jsonl",
        std::process::id()
    ))
}

fn capture_entries(path: &std::path::Path) -> Vec<DiagnosticEntry> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|entry: &DiagnosticEntry| entry.domain == DiagnosticDomain::Capture)
        .collect()
}

/// A manual failure — the tester closes the window Encore is capturing —
/// must be reconstructable from the log alone: it should show capture
/// reaching a healthy state, losing the source, and recovering once the
/// tester picks a source again, all timestamped and in order.
#[test]
fn a_source_loss_and_manual_retry_sequence_appears_in_the_log_in_order() {
    let path = scratch_log_path();
    let diagnostics = DiagnosticLog::open(path.clone());
    let service = service_with_diagnostics(
        Box::new(FakeBackend {
            fail_window: Arc::new(AtomicBool::new(false)),
            resize_count: Arc::new(AtomicUsize::new(0)),
            ready_status: SCFrameStatus::Complete,
        }),
        diagnostics,
    );

    service.switch_by_id("window:2").unwrap();
    service.mark_unavailable("window:2");
    service.retry().unwrap();

    let entries = capture_entries(&path);
    let states: Vec<&str> = entries.iter().map(|entry| entry.state.as_str()).collect();
    assert_eq!(states, vec!["capturing", "source_unavailable", "capturing"]);
    assert_eq!(entries[1].code.as_deref(), Some("source_unavailable"));
    assert_eq!(entries[0].code, None);
    assert_eq!(entries[2].code, None);
    // Entries are appended in the order they happened.
    assert!(entries[0].timestamp_unix_ms <= entries[1].timestamp_unix_ms);
    assert!(entries[1].timestamp_unix_ms <= entries[2].timestamp_unix_ms);

    let _ = std::fs::remove_file(&path);
}
