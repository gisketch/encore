const DEFAULT_AFTER_SAVE: &str = "nothing";
const VALID_AFTER_SAVE: [&str; 2] = ["reveal", "nothing"];

pub(super) fn default() -> String {
    DEFAULT_AFTER_SAVE.to_string()
}

/// Whether a value is one of the two after-save behaviors the settings
/// window and the replay dispatch understand. `Open editor` is deliberately
/// absent until PG-15.
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
