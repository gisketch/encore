use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri_plugin_global_shortcut::Shortcut;

const DEFAULT_SAVE_REPLAY: &str = "Cmd+Alt+R";
const DEFAULT_PAUSE_CAPTURE: &str = "Cmd+Alt+P";
const DEFAULT_OPEN_LIBRARY: &str = "Cmd+Alt+L";

/// Identifies one of the three rebindable hotkeys, independent of its
/// current accelerator string. Shared by the settings document (field
/// lookup), the `update_hotkey` command (parsed from the frontend's row
/// id), and the top-level registrar (which action to dispatch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyId {
    SaveReplay,
    PauseCapture,
    OpenLibrary,
}

impl HotkeyId {
    pub(crate) fn parse(id: &str) -> Result<Self, String> {
        match id {
            "save_replay" => Ok(Self::SaveReplay),
            "pause_capture" => Ok(Self::PauseCapture),
            "open_library" => Ok(Self::OpenLibrary),
            _ => Err("hotkey_unknown".into()),
        }
    }
}

/// The three rebindable global hotkeys' persisted accelerator strings, in
/// `tauri-plugin-global-shortcut` syntax (e.g. `"Cmd+Alt+R"`) so the
/// registrar never has to translate between a display format and a
/// registration format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkeys {
    #[serde(default = "default_save_replay")]
    pub save_replay: String,
    #[serde(default = "default_pause_capture")]
    pub pause_capture: String,
    #[serde(default = "default_open_library")]
    pub open_library: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            save_replay: DEFAULT_SAVE_REPLAY.to_string(),
            pause_capture: DEFAULT_PAUSE_CAPTURE.to_string(),
            open_library: DEFAULT_OPEN_LIBRARY.to_string(),
        }
    }
}

fn default_save_replay() -> String {
    DEFAULT_SAVE_REPLAY.to_string()
}

fn default_pause_capture() -> String {
    DEFAULT_PAUSE_CAPTURE.to_string()
}

fn default_open_library() -> String {
    DEFAULT_OPEN_LIBRARY.to_string()
}

impl Hotkeys {
    pub(crate) fn get(&self, id: HotkeyId) -> &str {
        match id {
            HotkeyId::SaveReplay => &self.save_replay,
            HotkeyId::PauseCapture => &self.pause_capture,
            HotkeyId::OpenLibrary => &self.open_library,
        }
    }

    pub(crate) fn set(&mut self, id: HotkeyId, value: String) {
        match id {
            HotkeyId::SaveReplay => self.save_replay = value,
            HotkeyId::PauseCapture => self.pause_capture = value,
            HotkeyId::OpenLibrary => self.open_library = value,
        }
    }
}

/// Whether an accelerator string is one `tauri-plugin-global-shortcut` can
/// parse. Used both to sanitize a freshly loaded document (a hand-edited or
/// future-schema value falls back to the default rather than failing the
/// whole document) and to reject an invalid chord from `update_hotkey`
/// before it ever reaches registration.
pub(crate) fn valid_accelerator(value: &str) -> bool {
    Shortcut::from_str(value).is_ok()
}

/// Falls back to each field's default independently when its value fails
/// `valid_accelerator`, so one corrupt field never discards the other two.
pub(super) fn sanitized(hotkeys: Hotkeys) -> Hotkeys {
    Hotkeys {
        save_replay: sanitized_one(hotkeys.save_replay, DEFAULT_SAVE_REPLAY),
        pause_capture: sanitized_one(hotkeys.pause_capture, DEFAULT_PAUSE_CAPTURE),
        open_library: sanitized_one(hotkeys.open_library, DEFAULT_OPEN_LIBRARY),
    }
}

fn sanitized_one(value: String, default: &str) -> String {
    if valid_accelerator(&value) {
        value
    } else {
        default.to_string()
    }
}
