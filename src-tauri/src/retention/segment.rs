#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetainedSegment {
    pub path: std::path::PathBuf,
    pub byte_size: u64,
    pub(super) source_id: Option<String>,
    pub(super) sort_time_ms: u64,
    pub(super) duration_ms: u64,
}

impl RetainedSegment {
    pub(crate) fn evidence_start_unix_ms(&self) -> u64 {
        self.sort_time_ms
    }

    pub(crate) fn evidence_end_unix_ms(&self) -> u64 {
        self.sort_time_ms.saturating_add(self.duration_ms)
    }

    pub(crate) fn source_id(&self) -> Option<&str> {
        self.source_id.as_deref()
    }
}
