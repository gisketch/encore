use super::CaptureService;
use crate::capture::settings::SettingsSnapshot;

/// The save-confirmation sound is a small, independently-persisted slice of
/// nothing to do with capture state, but the replay dispatch reads it at
/// the moment a save reaches the `saved` state, from outside the Settings
/// window. Deliberately separate from the after-save choice, so a tester
/// who wants no preview can still hear the confirmation (and vice versa).
impl CaptureService {
    pub fn save_sound(&self) -> bool {
        *self
            .0
            .save_sound
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Applies and persists a new save-sound choice. A plain `bool` has no
    /// invalid values to reject, unlike `appearance`/`after_save`.
    pub fn set_save_sound(&self, enabled: bool) -> Result<SettingsSnapshot, String> {
        *self
            .0
            .save_sound
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = enabled;
        super::persistence::persist(self);
        Ok(self.settings_snapshot())
    }
}
