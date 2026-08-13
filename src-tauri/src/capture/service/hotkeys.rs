use super::CaptureService;
use crate::capture::settings::{self, HotkeyId, Hotkeys, SettingsSnapshot};

/// Hotkeys are a small, independently-persisted slice of the settings
/// document, mirroring `appearance`/`after_save`: read/written through their
/// own methods rather than the capture snapshot. Unlike those, registration
/// with the OS happens outside this service entirely (the top-level
/// `hotkeys` registrar) — by the time `set_hotkey` is reached, the new
/// accelerator has already registered successfully, so this only has to
/// apply and persist the value.
impl CaptureService {
    pub fn hotkeys(&self) -> Hotkeys {
        self.0
            .hotkeys
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The current accelerator for a single hotkey, read by the registrar
    /// before a rebind so it knows what to re-register on failure.
    pub(crate) fn hotkey(&self, id: HotkeyId) -> String {
        self.hotkeys().get(id).to_string()
    }

    /// Validates, applies, and persists a single hotkey's accelerator.
    /// Rejects anything the registrar could not have parsed rather than
    /// writing a value the settings document would later have to sanitize
    /// away.
    pub(crate) fn set_hotkey(
        &self,
        id: HotkeyId,
        accelerator: String,
    ) -> Result<SettingsSnapshot, String> {
        if !settings::valid_hotkey_accelerator(&accelerator) {
            return Err("hotkey_invalid".into());
        }
        let mut hotkeys = self.hotkeys();
        hotkeys.set(id, accelerator);
        *self
            .0
            .hotkeys
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = hotkeys;
        super::persistence::persist(self);
        Ok(self.settings_snapshot())
    }
}
