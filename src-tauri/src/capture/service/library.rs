use super::CaptureService;

/// Opens the Library window (PG-10). Shared by the bar's library button
/// (via `open_library_window`), the `open_library` hotkey, and the tray's
/// "Open Library" item, all three of which route through
/// `hotkeys::dispatch` into this one method — so every entry point stays a
/// single implementation per PG-07's "same code path" contract. Revealing
/// the export folder in Finder is a separate, still-Finder-reveal action
/// now owned by `reveal_export_folder` below (the Library window's "Open
/// Folder ↗" button).
impl CaptureService {
    pub fn open_library(&self) -> Result<(), String> {
        let app = self
            .0
            .app
            .as_ref()
            .ok_or_else(|| "library_window_unavailable".to_string())?;
        crate::desktop::open_library_window(app)
    }

    /// Reveals the resolved save destination in Finder.
    pub fn reveal_export_folder(&self) -> Result<(), String> {
        let folder = self.resolved_save_destination();
        std::fs::create_dir_all(&folder).map_err(|_| "export_folder_unavailable".to_string())?;
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map(|_| ())
            .map_err(|_| "export_folder_open_failed".into())
    }
}
