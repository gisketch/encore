mod store;
#[cfg(test)]
mod tests;

use crate::capture::model::{CaptureSource, SourceKind};
use serde::{Deserialize, Serialize};

pub(crate) use store::SettingsStore;

const CURRENT_VERSION: u32 = 1;
const DEFAULT_RETENTION_MINUTES: u8 = 10;
const DEFAULT_APPEARANCE: &str = "system";
const VALID_APPEARANCES: [&str; 3] = ["light", "dark", "system"];

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
        }
    }
}

impl SettingsDocument {
    pub(crate) fn new(retention_minutes: u8, target: PersistedTarget, appearance: String) -> Self {
        Self {
            version: CURRENT_VERSION,
            retention_minutes,
            target,
            appearance,
        }
    }

    /// Normalizes a freshly parsed document: an out-of-range retention value
    /// or an unrecognized appearance (from a future schema or hand-edited
    /// file) each fall back to their default rather than failing the whole
    /// document.
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
}

impl From<&SettingsDocument> for SettingsSnapshot {
    fn from(document: &SettingsDocument) -> Self {
        Self {
            appearance: document.appearance.clone(),
            retention_minutes: document.retention_minutes,
            default_target: document.target.clone(),
        }
    }
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
