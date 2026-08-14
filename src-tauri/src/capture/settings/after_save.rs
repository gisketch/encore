const DEFAULT_AFTER_SAVE: &str = "preview";
const VALID_AFTER_SAVE: [&str; 4] = ["preview", "reveal", "nothing", "open_editor"];

pub(super) fn default() -> String {
    DEFAULT_AFTER_SAVE.to_string()
}

/// Whether a value is one of the four after-save behaviors the settings
/// window and the replay dispatch understand. `open_editor` (PG-15) opens
/// the just-saved replay in the Editor window; `preview` (PP-01) shows the
/// post-save preview and is the default for fresh installs.
pub(crate) fn valid(value: &str) -> bool {
    VALID_AFTER_SAVE.contains(&value)
}

/// Falls back to the default for anything not in `valid`: an unrecognized
/// choice from a future schema or a hand-edited file. Any already-persisted
/// valid value — including the pre-PP-01 default `nothing` — is returned
/// untouched, so changing the default never rewrites an existing choice.
pub(super) fn sanitized(value: String) -> String {
    if valid(&value) {
        value
    } else {
        default()
    }
}
