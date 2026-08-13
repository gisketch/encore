use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReplayState {
    Unavailable,
    Preparing,
    Saved,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ShortcutRegistrationState {
    Unavailable,
    Registered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShortcutRegistrationSnapshot {
    pub state: ShortcutRegistrationState,
    pub error_code: Option<String>,
}

impl Default for ShortcutRegistrationSnapshot {
    fn default() -> Self {
        Self {
            state: ShortcutRegistrationState::Unavailable,
            error_code: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingReplaySnapshot {
    pub id: String,
    pub created_at_unix_ms: u64,
    pub segment_count: u64,
    pub total_bytes: u64,
    pub evidence_start_unix_ms: u64,
    pub evidence_end_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedReplaySnapshot {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplaySnapshot {
    pub state: ReplayState,
    pub pending: Option<PendingReplaySnapshot>,
    pub saved: Option<SavedReplaySnapshot>,
    pub error_code: Option<String>,
    pub shortcut: ShortcutRegistrationSnapshot,
}
