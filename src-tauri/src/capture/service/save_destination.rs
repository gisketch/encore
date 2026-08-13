use super::CaptureService;
use crate::capture::settings::SettingsSnapshot;
use std::{path::PathBuf, sync::Arc};

/// Save destination is a small, independently-persisted slice of the
/// settings document, mirroring `appearance`/`default_target`: read/written
/// through its own methods rather than the capture snapshot. Unlike
/// `default_target` it is also read by the replay pipeline at save time —
/// `resolved_save_destination` is the seam that gives every new save the
/// CURRENT value instead of one baked in at construction.
impl CaptureService {
    /// The raw persisted value. `None` means "use the default folder";
    /// callers that need an actual path should use
    /// `resolved_save_destination` instead.
    pub fn save_destination(&self) -> Option<PathBuf> {
        self.0
            .save_destination
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The folder replays are saved to right now: the persisted custom
    /// folder, or the default `Movies/Encore` folder when none is set.
    pub fn resolved_save_destination(&self) -> PathBuf {
        self.save_destination()
            .unwrap_or_else(|| self.0.default_destination.clone())
    }

    /// Builds a `ReplayService` destination lookup bound to this service, as
    /// a named method rather than a closure literal at the `lib.rs` call
    /// site, so its setup stays a straight-line list of steps.
    pub fn destination_lookup(&self) -> Arc<dyn Fn() -> PathBuf + Send + Sync> {
        let service = self.clone();
        Arc::new(move || service.resolved_save_destination())
    }

    /// Validates that `path` exists or can be created, applies it, and
    /// persists it. `None` resets to the default destination. Validation
    /// failure leaves the previously persisted value untouched.
    pub fn set_save_destination(&self, path: Option<PathBuf>) -> Result<SettingsSnapshot, String> {
        let target = path
            .clone()
            .unwrap_or_else(|| self.0.default_destination.clone());
        std::fs::create_dir_all(&target)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        *self
            .0
            .save_destination
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = path;
        super::persistence::persist(self);
        Ok(self.settings_snapshot())
    }
}
