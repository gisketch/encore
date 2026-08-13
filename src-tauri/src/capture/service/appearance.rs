use super::CaptureService;
use crate::capture::settings;

/// Appearance is a small, independently-persisted slice of the settings
/// document: read/written through its own methods rather than the capture
/// snapshot, since it has nothing to do with capture state and every window
/// (not just the bar) needs to read and react to it.
impl CaptureService {
    pub fn appearance(&self) -> String {
        self.0
            .appearance
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Validates, applies, and persists a new appearance choice. Rejects
    /// anything outside light/dark/system rather than writing a value the
    /// settings document would later have to sanitize away.
    pub fn set_appearance(&self, value: String) -> Result<String, String> {
        if !settings::valid_appearance(&value) {
            return Err("appearance_invalid".into());
        }
        *self
            .0
            .appearance
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = value.clone();
        super::persistence::persist(self);
        Ok(value)
    }
}
