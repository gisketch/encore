//! The tray menu's status line.
//!
//! A disabled first item that answers the question the menu bar exists to
//! answer at a glance: is Encore actually recording right now? Kept in its
//! own module so the state-to-text mapping is a pure function with tests,
//! and so `tray` itself stays free of the branching.

use crate::capture::CaptureState;

/// What the disabled status item reads for a given capture state.
///
/// Deliberately plain-language and honest: only `Capturing` may claim
/// "Recording", because that is the one state in which frames are actually
/// reaching the rolling buffer. Every other state says what is true
/// instead, so the menu can never imply evidence is being kept when it is
/// not.
pub(crate) fn status_label(capture: CaptureState) -> &'static str {
    match capture {
        CaptureState::Capturing => "● Recording",
        CaptureState::Starting => "Starting…",
        CaptureState::Recovering => "Recovering…",
        CaptureState::Paused => "Paused — not recording",
        CaptureState::Stopped => "Not recording",
        CaptureState::SourceUnavailable => "Source unavailable",
        CaptureState::Failed => "Capture failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the line: "Recording" appears for exactly one
    /// state, so a tester reading the menu can trust it.
    #[test]
    fn only_capturing_claims_to_be_recording() {
        for capture in [
            CaptureState::Stopped,
            CaptureState::Starting,
            CaptureState::Paused,
            CaptureState::Recovering,
            CaptureState::SourceUnavailable,
            CaptureState::Failed,
        ] {
            assert!(
                !status_label(capture).contains("● Recording"),
                "{capture:?} claimed to be recording"
            );
        }
        assert_eq!(status_label(CaptureState::Capturing), "● Recording");
    }

    /// A paused capture is the state most easily mistaken for a running
    /// one, so its text says both what it is and what it is not.
    #[test]
    fn paused_says_it_is_not_recording() {
        assert!(status_label(CaptureState::Paused).contains("not recording"));
    }

    #[test]
    fn every_state_has_non_empty_text() {
        for capture in [
            CaptureState::Stopped,
            CaptureState::Starting,
            CaptureState::Capturing,
            CaptureState::Paused,
            CaptureState::Recovering,
            CaptureState::SourceUnavailable,
            CaptureState::Failed,
        ] {
            assert!(!status_label(capture).is_empty(), "{capture:?} had no text");
        }
    }
}
