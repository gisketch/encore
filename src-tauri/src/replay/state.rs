use super::{
    PendingReplaySnapshot, ReplaySnapshot, ReplayState, SavedReplaySnapshot,
    ShortcutRegistrationSnapshot,
};
use crate::{capture::EvidenceCaptureFacts, packager::SavedReplay, retention::SegmentLease};

pub(super) struct ReplayServiceState {
    pub(super) next_id: u64,
    pub(super) pending: Option<PendingReplay>,
    pub(super) exporting_id: Option<String>,
    pub(super) saved: Option<SavedReplay>,
    pub(super) error_code: Option<String>,
    pub(super) shortcut: ShortcutRegistrationSnapshot,
}

pub(super) struct PendingReplay {
    pub(super) lease: SegmentLease,
    pub(super) snapshot: PendingReplaySnapshot,
    pub(super) capture: EvidenceCaptureFacts,
}

pub(super) fn pending_snapshot(
    lease: &SegmentLease,
    next_id: u64,
    created_at_unix_ms: u64,
) -> Option<PendingReplaySnapshot> {
    let segments = lease.segments();
    let first = segments.first()?;
    let evidence_start_unix_ms = segments
        .iter()
        .map(|segment| segment.evidence_start_unix_ms())
        .min()
        .unwrap_or_else(|| first.evidence_start_unix_ms());
    let evidence_end_unix_ms = segments
        .iter()
        .map(|segment| segment.evidence_end_unix_ms())
        .max()
        .unwrap_or_else(|| first.evidence_end_unix_ms());
    Some(PendingReplaySnapshot {
        id: format!("replay-{next_id}"),
        created_at_unix_ms,
        segment_count: segments.len().try_into().unwrap_or(u64::MAX),
        total_bytes: segments.iter().fold(0_u64, |total, segment| {
            total.saturating_add(segment.byte_size)
        }),
        evidence_start_unix_ms,
        evidence_end_unix_ms,
    })
}

pub(super) fn record_failure(state: &mut ReplayServiceState, code: &str) -> String {
    state.error_code = Some(code.to_owned());
    code.to_owned()
}

pub(super) fn snapshot(state: &ReplayServiceState) -> ReplaySnapshot {
    let pending = state
        .pending
        .as_ref()
        .map(|pending| pending.snapshot.clone());
    let saved = state.saved.as_ref().map(|replay| SavedReplaySnapshot {
        id: replay.id.clone(),
        display_name: replay.display_name.clone(),
    });
    let replay_state = if state.exporting_id.is_some() {
        ReplayState::Preparing
    } else if pending.is_none() && state.error_code.as_deref() == Some("replay_unavailable") {
        ReplayState::Unavailable
    } else if pending.is_some() && state.error_code.is_some() {
        ReplayState::Failed
    } else if saved.is_some() {
        ReplayState::Saved
    } else if state.error_code.is_some() {
        ReplayState::Failed
    } else {
        ReplayState::Unavailable
    };
    ReplaySnapshot {
        state: replay_state,
        pending,
        saved,
        error_code: state.error_code.clone(),
        shortcut: state.shortcut.clone(),
    }
}
