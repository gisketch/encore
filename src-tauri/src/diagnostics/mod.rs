#[cfg(test)]
mod tests;
mod writer;

use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use writer::DiagnosticWriter;

/// Size cap for the active diagnostic log file before it rotates to
/// `<path>.1`. Low single-digit megabytes comfortably covers a full tester
/// day of lifecycle transitions without growing unbounded.
const DEFAULT_MAX_BYTES: u64 = 2 * 1024 * 1024;

/// The four boundaries the log must be able to explain a failed day from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticDomain {
    Permission,
    Capture,
    Retention,
    Export,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEntry {
    pub timestamp_unix_ms: u64,
    pub domain: DiagnosticDomain,
    pub state: String,
    pub code: Option<String>,
}

/// Shared, cheap-to-clone handle to the local lifecycle/failure log. Every
/// service that needs to explain "why did today's capture stop" holds one of
/// these instead of threading a logger through its call graph. Appends take
/// a short lock around a single buffered write; a write failure (missing
/// directory, permission error, disk full) degrades to a silent no-op and
/// must never break capture, retention, or export.
#[derive(Clone)]
pub struct DiagnosticLog(Arc<Mutex<DiagnosticWriter>>);

impl DiagnosticLog {
    /// Opens (or creates) the JSON Lines log at `path`, rotating the active
    /// file to `<path>.1` once it would exceed the default size cap.
    pub fn open(path: PathBuf) -> Self {
        Self::with_cap(path, DEFAULT_MAX_BYTES)
    }

    pub(crate) fn with_cap(path: PathBuf, max_bytes: u64) -> Self {
        Self(Arc::new(Mutex::new(DiagnosticWriter::open(
            path, max_bytes,
        ))))
    }

    /// A handle that never touches disk. Used where no app log directory is
    /// available yet (most unit tests, and any construction path that must
    /// not depend on filesystem state).
    #[cfg(test)]
    pub fn disabled() -> Self {
        Self(Arc::new(Mutex::new(DiagnosticWriter::disabled())))
    }

    pub fn record(&self, domain: DiagnosticDomain, state: impl Into<String>, code: Option<String>) {
        let entry = DiagnosticEntry {
            timestamp_unix_ms: now_unix_ms(),
            domain,
            state: state.into(),
            code,
        };
        let mut writer = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        writer.append(&entry);
    }
}

/// Renders a `#[serde(rename_all = "snake_case")]` unit-ish enum to its bare
/// state label (e.g. `CaptureState::SourceUnavailable` ->
/// `source_unavailable`), so callers can log the same typed states already
/// shown in the webview without hand-maintaining a parallel string mapping.
pub fn label<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "unknown".to_string(),
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
