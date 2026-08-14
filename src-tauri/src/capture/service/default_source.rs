use super::CaptureService;
use crate::capture::settings::{PersistedTarget, SettingsDocument, SettingsSnapshot};

/// Default source is the launch-time capture target, shown and editable in
/// the Settings window's Recording section. Unlike appearance, it is also
/// written by the bar's live source switch (`persistence::persist`) — both
/// paths converge on the same in-memory cache and on-disk field, and the
/// most recent write is what the next launch (and the Settings window) sees.
impl CaptureService {
    pub fn default_target(&self) -> PersistedTarget {
        self.0
            .default_target
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Updates the in-memory default-target cache without touching disk.
    /// Used both here and by `persistence::persist` after a live switch, so
    /// the cache never drifts from whichever path wrote most recently.
    pub(super) fn sync_default_target(&self, target: PersistedTarget) {
        *self
            .0
            .default_target
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = target;
    }

    /// Resolves `source_id` against the currently available sources and
    /// persists it as the launch-time default, without touching the live
    /// capture stream. Mirrors `switch_by_id`'s lookup but never calls
    /// `switch_source`, per the spec: the bar stays the only live switcher.
    pub fn set_default_source_by_id(&self, source_id: &str) -> Result<SettingsSnapshot, String> {
        let source = self
            .list_sources()?
            .into_iter()
            .find(|candidate| candidate.id == source_id)
            .ok_or_else(|| "source_unavailable".to_string())?;
        let target = PersistedTarget::from_source(&source);
        self.sync_default_target(target.clone());
        self.persist_default_target_only(&target);
        Ok(self.settings_snapshot())
    }

    fn persist_default_target_only(&self, target: &PersistedTarget) {
        let document = SettingsDocument::new(
            self.snapshot().retention.minutes,
            target.clone(),
            self.appearance(),
            self.save_destination(),
            self.after_save(),
            self.hotkeys(),
            self.save_sound(),
        );
        let _ = self.0.settings.save(&document);
    }
}
