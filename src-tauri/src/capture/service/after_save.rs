use super::CaptureService;
use crate::capture::settings::{self, SettingsSnapshot};

/// After-save behavior is a small, independently-persisted slice of the
/// settings document: whether a completed save reveals the bundle in
/// Finder or stays quiet. Consumed by the replay dispatch at the point a
/// save reaches the `saved` state, mirroring `appearance`'s pattern of a
/// dedicated read/write pair instead of living on the capture snapshot.
impl CaptureService {
    pub fn after_save(&self) -> String {
        self.0
            .after_save
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Validates, applies, and persists a new after-save choice. Rejects
    /// anything outside `reveal`/`nothing` rather than writing a value the
    /// settings document would later have to sanitize away.
    pub fn set_after_save(&self, value: String) -> Result<SettingsSnapshot, String> {
        if !settings::valid_after_save(&value) {
            return Err("after_save_invalid".into());
        }
        *self
            .0
            .after_save
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = value;
        super::persistence::persist(self);
        Ok(self.settings_snapshot())
    }
}
