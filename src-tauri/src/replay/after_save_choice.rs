/// The persisted after-save choice, resolved from its stored string into
/// the one action the dispatch should take. Lives in its own file so the
/// string-to-action routing is directly testable without an `AppHandle`,
/// and so new choices add their branching here rather than in the already
/// committed `after_save_dispatch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AfterSaveAction {
    Reveal,
    OpenEditor,
    Preview,
    Nothing,
}

/// Maps a persisted choice to its action. Anything unrecognized (a future
/// schema, a hand-edited settings file that slipped past sanitization)
/// resolves to `Nothing`, matching the historical catch-all behavior.
pub(super) fn action_for(choice: &str) -> AfterSaveAction {
    match choice {
        "reveal" => AfterSaveAction::Reveal,
        "open_editor" => AfterSaveAction::OpenEditor,
        "preview" => AfterSaveAction::Preview,
        _ => AfterSaveAction::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_persisted_choice_routes_to_its_own_action() {
        assert_eq!(action_for("reveal"), AfterSaveAction::Reveal);
        assert_eq!(action_for("open_editor"), AfterSaveAction::OpenEditor);
        assert_eq!(action_for("preview"), AfterSaveAction::Preview);
        assert_eq!(action_for("nothing"), AfterSaveAction::Nothing);
    }

    #[test]
    fn an_unrecognized_choice_falls_back_to_doing_nothing() {
        for choice in ["", "delete", "Preview", "open editor"] {
            assert_eq!(action_for(choice), AfterSaveAction::Nothing);
        }
    }
}
