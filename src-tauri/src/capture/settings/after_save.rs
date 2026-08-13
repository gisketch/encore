const DEFAULT_AFTER_SAVE: &str = "nothing";
const VALID_AFTER_SAVE: [&str; 3] = ["reveal", "nothing", "open_editor"];

pub(super) fn default() -> String {
    DEFAULT_AFTER_SAVE.to_string()
}

/// Whether a value is one of the three after-save behaviors the settings
/// window and the replay dispatch understand. `open_editor` (PG-15) opens
/// the just-saved replay in the Editor window; the default stays `nothing`.
pub(crate) fn valid(value: &str) -> bool {
    VALID_AFTER_SAVE.contains(&value)
}

/// Falls back to the default for anything not in `valid`: an unrecognized
/// choice from a future schema or a hand-edited file.
pub(super) fn sanitized(value: String) -> String {
    if valid(&value) {
        value
    } else {
        default()
    }
}
