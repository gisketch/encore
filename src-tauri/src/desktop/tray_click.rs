//! Double-click on the menu bar icon shows the action bar.
//!
//! `TrayIconEvent::DoubleClick` exists in Tauri's enum but the macOS
//! backend never emits it — `tray-icon`'s macOS implementation sends only
//! `Click`, `Enter`, `Leave`, and `Move` (double-click is Windows-only).
//! It does, however, send the left `Click`/`Down` *before* it opens the
//! attached menu, so the gesture is recoverable by timing two presses
//! ourselves, which is what this module does.
//!
//! The menu item "Show Action Bar" remains the guaranteed restore path;
//! this is only an accelerator on top of it.

use super::tray;
use std::{
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconEvent},
    Wry,
};

/// How close two presses must be to count as one double-click. macOS's own
/// default double-click interval is 500ms, so this matches what a user's
/// hands are already calibrated to.
const THRESHOLD: Duration = Duration::from_millis(500);

static LAST_PRESS: Mutex<Option<Instant>> = Mutex::new(None);

/// Whether `now` completes a double-click that began at `previous`. Pure,
/// so the timing rule is testable without a tray, a clock, or an app.
pub(crate) fn completes_double_click(
    previous: Option<Instant>,
    now: Instant,
    threshold: Duration,
) -> bool {
    previous.is_some_and(|previous| now.duration_since(previous) <= threshold)
}

/// Records a press and reports whether it completed a double-click.
/// Completing one clears the stored press, so three rapid clicks are one
/// double-click plus a fresh first press rather than two overlapping
/// double-clicks.
fn register_press(now: Instant) -> bool {
    let mut last = LAST_PRESS.lock().unwrap_or_else(PoisonError::into_inner);
    let completed = completes_double_click(*last, now, THRESHOLD);
    *last = if completed { None } else { Some(now) };
    completed
}

/// Tray icon event handler. Only the left button's press edge is
/// interesting: the release arrives after the menu has opened, and the
/// other buttons belong to the menu.
pub(crate) fn handle(icon: &TrayIcon<Wry>, event: TrayIconEvent) {
    let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Down,
        ..
    } = event
    else {
        return;
    };
    if register_press(Instant::now()) {
        tray::show_action_bar(icon.app_handle());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_first_press_is_not_a_double_click() {
        assert!(!completes_double_click(None, Instant::now(), THRESHOLD));
    }

    #[test]
    fn a_second_press_inside_the_threshold_completes_one() {
        let first = Instant::now();
        let second = first + Duration::from_millis(200);

        assert!(completes_double_click(Some(first), second, THRESHOLD));
    }

    #[test]
    fn a_second_press_after_the_threshold_is_just_another_first_press() {
        let first = Instant::now();
        let late = first + THRESHOLD + Duration::from_millis(1);

        assert!(!completes_double_click(Some(first), late, THRESHOLD));
    }

    /// A press exactly on the boundary counts, so the rule has no gap a
    /// user could land in and see nothing happen.
    #[test]
    fn a_press_exactly_on_the_threshold_counts() {
        let first = Instant::now();

        assert!(completes_double_click(
            Some(first),
            first + THRESHOLD,
            THRESHOLD
        ));
    }

    /// Three rapid presses must fire exactly once: the third starts a new
    /// gesture rather than completing a second, overlapping double-click.
    #[test]
    fn three_rapid_presses_fire_exactly_one_double_click() {
        let base = Instant::now();
        let mut previous = None;
        let mut fired = 0;
        for step in [0, 100, 200] {
            let now = base + Duration::from_millis(step);
            let completed = completes_double_click(previous, now, THRESHOLD);
            fired += usize::from(completed);
            previous = if completed { None } else { Some(now) };
        }

        assert_eq!(fired, 1);
    }
}
