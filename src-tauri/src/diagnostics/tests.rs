use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

fn scratch_path() -> PathBuf {
    let id = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "encore-diagnostics-test-{}-{id}.jsonl",
        std::process::id()
    ))
}

fn read_lines(path: &std::path::Path) -> Vec<DiagnosticEntry> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn a_recorded_entry_carries_timestamp_domain_state_and_code() {
    let path = scratch_path();
    let log = DiagnosticLog::open(path.clone());

    log.record(
        DiagnosticDomain::Capture,
        "source_unavailable",
        Some("source_unavailable".into()),
    );

    let entries = read_lines(&path);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].timestamp_unix_ms > 0);
    assert_eq!(entries[0].domain, DiagnosticDomain::Capture);
    assert_eq!(entries[0].state, "source_unavailable");
    assert_eq!(entries[0].code.as_deref(), Some("source_unavailable"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_record_without_a_code_serializes_a_null_code() {
    let path = scratch_path();
    let log = DiagnosticLog::open(path.clone());

    log.record(DiagnosticDomain::Permission, "granted", None);

    let entries = read_lines(&path);
    assert_eq!(entries[0].code, None);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn exceeding_the_cap_rotates_the_active_file_to_a_single_predecessor() {
    let path = scratch_path();
    let rotated = super::writer::rotated_path_for(&path);
    // Small enough that a handful of records overflow it repeatedly.
    let log = DiagnosticLog::with_cap(path.clone(), 120);

    for index in 0..20 {
        log.record(DiagnosticDomain::Capture, format!("state-{index}"), None);
    }

    assert!(path.exists());
    assert!(rotated.exists());
    let active_len = std::fs::metadata(&path).unwrap().len();
    assert!(active_len <= 120, "active file should stay within the cap");

    // The newest record always lands in the active file, never lost.
    let active_entries = read_lines(&path);
    assert_eq!(active_entries.last().unwrap().state, "state-19");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&rotated);
}

#[test]
fn a_failed_rotation_drops_old_content_instead_of_growing_without_bound() {
    let path = scratch_path();
    let rotated = super::writer::rotated_path_for(&path);
    // A non-empty directory at the rotated path makes the rename fail
    // deterministically, standing in for any filesystem that refuses it.
    std::fs::create_dir_all(rotated.join("blocker")).unwrap();
    let log = DiagnosticLog::with_cap(path.clone(), 120);

    for index in 0..20 {
        log.record(DiagnosticDomain::Capture, format!("state-{index}"), None);
    }

    // The cap holds even though no predecessor could be kept, and the
    // newest record still lands in the active file.
    let active_len = std::fs::metadata(&path).unwrap().len();
    assert!(active_len <= 120, "active file should stay within the cap");
    assert_eq!(read_lines(&path).last().unwrap().state, "state-19");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(&rotated);
}

#[test]
fn a_write_failure_degrades_to_a_silent_no_op() {
    // A regular file can never be a valid parent directory, so opening a log
    // underneath one deterministically fails without touching real disk
    // errors like a full volume or a permission bit.
    let blocker = scratch_path();
    std::fs::write(&blocker, b"not a directory").unwrap();
    let unwritable = blocker.join("sub").join("diagnostics.jsonl");

    let log = DiagnosticLog::open(unwritable.clone());
    // Must not panic and must not fabricate the path it couldn't open.
    log.record(
        DiagnosticDomain::Export,
        "failed",
        Some("export_failed".into()),
    );

    assert!(!unwritable.exists());
    let _ = std::fs::remove_file(&blocker);
}

#[test]
fn a_disabled_log_never_touches_disk() {
    let log = DiagnosticLog::disabled();
    log.record(
        DiagnosticDomain::Retention,
        "failed",
        Some("retention_scan_failed".into()),
    );
    // No path was ever given, so there is nothing to assert on disk beyond
    // the absence of a panic above.
}

#[test]
fn label_renders_the_bare_snake_case_state() {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Sample {
        SourceUnavailable,
    }

    assert_eq!(label(&Sample::SourceUnavailable), "source_unavailable");
}
