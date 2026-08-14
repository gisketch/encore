mod after_save;
mod hotkeys;
mod store;
#[cfg(test)]
mod tests;

use crate::capture::model::{CaptureSource, SourceKind};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) use after_save::valid as valid_after_save;
pub use hotkeys::Hotkeys;
pub(crate) use hotkeys::{valid_accelerator as valid_hotkey_accelerator, HotkeyId};
pub(crate) use store::SettingsStore;

const CURRENT_VERSION: u32 = 1;
const DEFAULT_RETENTION_MINUTES: u8 = 10;
const DEFAULT_APPEARANCE: &str = "system";
const VALID_APPEARANCES: [&str; 3] = ["light", "dark", "system"];
const DEFAULT_SAVE_SOUND: bool = true;

/// Best-effort capture target identity that survives relaunch. Windows
/// resolve by app bundle plus title; there is no stable id across launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PersistedTarget {
    #[default]
    Display,
    Window {
        bundle_id: String,
        title: String,
    },
}

impl PersistedTarget {
    pub(crate) fn from_source(source: &CaptureSource) -> Self {
        match source.kind {
            SourceKind::Display => Self::Display,
            SourceKind::Window => Self::Window {
                bundle_id: source.bundle_id.clone().unwrap_or_default(),
                title: source.title.clone().unwrap_or_default(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SettingsDocument {
    #[serde(default = "current_version")]
    pub version: u32,
    #[serde(default = "default_retention_minutes")]
    pub retention_minutes: u8,
    #[serde(default)]
    pub target: PersistedTarget,
    #[serde(default = "default_appearance")]
    pub appearance: String,
    /// Absent means "use the default `Movies/Encore` folder"; a resolved
    /// current-launch default is never written here, so relocating the
    /// default folder later does not silently strand existing users on it.
    #[serde(default)]
    pub save_destination: Option<PathBuf>,
    #[serde(default = "after_save::default")]
    pub after_save: String,
    #[serde(default)]
    pub hotkeys: Hotkeys,
    /// Whether the floating bar is hidden in favor of a tray-only presence.
    /// A plain `bool` needs no sanitization: a missing or wrongly-typed
    /// value falls back to `false` (the historical always-visible bar)
    /// through `#[serde(default)]`/the corrupt-file catch-all in
    /// `SettingsStore::load`.
    #[serde(default)]
    pub menu_bar_mode: bool,
    /// Whether a successful save plays the confirmation chime. Defaults to
    /// `true`, so it needs its own default function rather than
    /// `#[serde(default)]`: a settings file written before PP-03 (and the
    /// corrupt-file catch-all in `SettingsStore::load`) must land on the
    /// sound being on, not off. Like `menu_bar_mode`, a plain `bool` has no
    /// invalid values to sanitize.
    #[serde(default = "default_save_sound")]
    pub save_sound: bool,
}

fn current_version() -> u32 {
    CURRENT_VERSION
}

fn default_retention_minutes() -> u8 {
    DEFAULT_RETENTION_MINUTES
}

fn default_appearance() -> String {
    DEFAULT_APPEARANCE.to_string()
}

fn default_save_sound() -> bool {
    DEFAULT_SAVE_SOUND
}

/// Whether a value is one of the three appearance choices the settings
/// window and the design system's theme resolution understand.
pub(crate) fn valid_appearance(value: &str) -> bool {
    VALID_APPEARANCES.contains(&value)
}

impl Default for SettingsDocument {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            retention_minutes: DEFAULT_RETENTION_MINUTES,
            target: PersistedTarget::Display,
            appearance: DEFAULT_APPEARANCE.to_string(),
            save_destination: None,
            after_save: after_save::default(),
            hotkeys: Hotkeys::default(),
            menu_bar_mode: false,
            save_sound: DEFAULT_SAVE_SOUND,
        }
    }
}

impl SettingsDocument {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        retention_minutes: u8,
        target: PersistedTarget,
        appearance: String,
        save_destination: Option<PathBuf>,
        after_save: String,
        hotkeys: Hotkeys,
        menu_bar_mode: bool,
        save_sound: bool,
    ) -> Self {
        Self {
            version: CURRENT_VERSION,
            retention_minutes,
            target,
            appearance,
            save_destination,
            after_save,
            hotkeys,
            menu_bar_mode,
            save_sound,
        }
    }

    /// Normalizes a freshly parsed document: an out-of-range retention value
    /// or an unrecognized appearance/after-save choice (from a future schema
    /// or hand-edited file) each fall back to their default rather than
    /// failing the whole document.
    fn sanitized(self) -> Self {
        let retention_minutes = match self.retention_minutes {
            5 | 10 => self.retention_minutes,
            _ => DEFAULT_RETENTION_MINUTES,
        };
        let appearance = if valid_appearance(&self.appearance) {
            self.appearance
        } else {
            DEFAULT_APPEARANCE.to_string()
        };
        Self {
            version: CURRENT_VERSION,
            retention_minutes,
            target: self.target,
            appearance,
            save_destination: self.save_destination,
            after_save: after_save::sanitized(self.after_save),
            hotkeys: hotkeys::sanitized(self.hotkeys),
            menu_bar_mode: self.menu_bar_mode,
            save_sound: self.save_sound,
        }
    }
}

/// The subset of the settings document the frontend reads/writes today.
/// Grows as later tickets add sections; kept separate from
/// `SettingsDocument` so the on-disk shape can evolve without widening the
/// frontend contract.
#[derive(Debug, Clone, Serialize)]
pub struct SettingsSnapshot {
    pub appearance: String,
    pub retention_minutes: u8,
    pub default_target: PersistedTarget,
    /// The resolved destination replays are saved to right now: the
    /// persisted custom folder, or the default `Movies/Encore` folder when
    /// none is set. Always an absolute path, never `None`, so the frontend
    /// never has to know about the default-folder fallback itself.
    pub save_destination: PathBuf,
    pub after_save: String,
    pub hotkeys: Hotkeys,
    pub menu_bar_mode: bool,
    pub save_sound: bool,
}

/// Resolves a persisted target against currently available sources. Returns
/// `None` for the default-display target or when a persisted window cannot
/// be found, in which case the caller falls back to the default display.
pub(crate) fn resolve_target(
    sources: &[CaptureSource],
    target: &PersistedTarget,
) -> Option<CaptureSource> {
    let PersistedTarget::Window { bundle_id, title } = target else {
        return None;
    };
    sources
        .iter()
        .find(|source| {
            source.kind == SourceKind::Window
                && source.bundle_id.as_deref() == Some(bundle_id.as_str())
                && source.title.as_deref() == Some(title.as_str())
        })
        .cloned()
}
