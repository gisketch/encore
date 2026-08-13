use super::CaptureService;
use crate::capture::settings::SettingsSnapshot;

/// Menu-bar mode is a small, independently-persisted slice of the settings
/// document, mirroring `appearance`'s pattern: it has nothing to do with
/// capture state, but the desktop shell (window visibility, tray menu
/// shape) needs to read and react to it from outside the Settings window.
impl CaptureService {
    pub fn menu_bar_mode(&self) -> bool {
        *self
            .0
            .menu_bar_mode
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Applies and persists a new menu-bar-mode choice. A plain `bool` has
    /// no invalid values to reject, unlike `appearance`/`after_save`.
    pub fn set_menu_bar_mode(&self, enabled: bool) -> Result<SettingsSnapshot, String> {
        *self
            .0
            .menu_bar_mode
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = enabled;
        super::persistence::persist(self);
        Ok(self.settings_snapshot())
    }
}
