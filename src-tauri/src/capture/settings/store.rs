use super::SettingsDocument;
use std::{fs, path::PathBuf};

/// Loads and atomically persists the settings document at a fixed path. The
/// path is resolved once (via Tauri's path API in production, or a fixture
/// path in tests) and injected, so this stays testable without a live app.
#[derive(Clone)]
pub(crate) struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Missing or corrupt settings never block startup: both cases yield
    /// defaults instead of an error.
    pub(crate) fn load(&self) -> SettingsDocument {
        fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SettingsDocument>(&bytes).ok())
            .map(SettingsDocument::sanitized)
            .unwrap_or_default()
    }

    /// Writes the document to a sibling temp file and renames it into place,
    /// so a reader never observes a partially written document.
    pub(crate) fn save(&self, document: &SettingsDocument) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(document)
            .map_err(|_| "settings_encode_failed".to_string())?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "settings_path_invalid".to_string())?;
        fs::create_dir_all(parent).map_err(|_| "settings_directory_unavailable".to_string())?;
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "settings_path_invalid".to_string())?;
        let tmp_path = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
        fs::write(&tmp_path, &bytes).map_err(|_| "settings_write_failed".to_string())?;
        fs::rename(&tmp_path, &self.path).map_err(|_| {
            let _ = fs::remove_file(&tmp_path);
            "settings_write_failed".to_string()
        })
    }
}
