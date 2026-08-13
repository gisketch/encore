use super::CaptureService;

/// Reveals the resolved save destination in Finder. Shared by the
/// `open_export_folder` command (the bar's library button) and the
/// `open_library` hotkey dispatch, so both routes stay a single
/// implementation per PG-07's "same code path as the bar's library button"
/// contract.
impl CaptureService {
    pub fn open_library(&self) -> Result<(), String> {
        let folder = self.resolved_save_destination();
        std::fs::create_dir_all(&folder).map_err(|_| "export_folder_unavailable".to_string())?;
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map(|_| ())
            .map_err(|_| "export_folder_open_failed".into())
    }
}
