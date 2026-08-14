mod after_save;
mod after_save_choice;
mod after_save_dispatch;
mod model;
mod reveal;
mod service;
mod shortcut;
mod state;
#[cfg(test)]
mod tests;

pub(crate) use model::{
    PendingReplaySnapshot, ReplaySnapshot, ReplayState, SavedReplaySnapshot,
    ShortcutRegistrationSnapshot, ShortcutRegistrationState,
};
/// Re-exported for `library::reveal`, which reveals one bundle through the
/// same `open -R` helper the replay module's own reveal path uses.
pub(crate) use reveal::reveal_in_finder;
pub(crate) use service::{ReplayDispatch, ReplayService};
pub(crate) use shortcut::{emit_snapshot, retry_and_emit, reveal_and_emit, trigger_and_emit};

#[cfg(test)]
use service::{ReplayClock, ReplayTime};
