use crate::capture::{CaptureService, CaptureState, HotkeyId, SettingsSnapshot};
use crate::hotkeys;
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Listener, Manager, Wry,
};

const CAPTURE_STATE_CHANGED_EVENT: &str = "capture-state-changed";

/// Handle to the tray icon, kept as managed state so its menu can be
/// rebuilt in place (mode toggle, capture-state change) without tearing
/// down and recreating the tray itself.
struct TrayHandle(TrayIcon<Wry>);

/// Every action the tray menu can route to. Kept separate from the actual
/// `MenuItem`s so `menu_actions` below stays pure and directly testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayAction {
    Show,
    Hide,
    SaveReplay,
    Pause,
    Resume,
    OpenLibrary,
    Settings,
    ShowFloatingBar,
    Quit,
}

impl TrayAction {
    fn id(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Hide => "hide",
            Self::SaveReplay => "save_replay",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::OpenLibrary => "open_library",
            Self::Settings => "settings",
            Self::ShowFloatingBar => "show_floating_bar",
            Self::Quit => "quit",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Show => "Show Encore",
            Self::Hide => "Hide Encore",
            Self::SaveReplay => "Save Replay",
            Self::Pause => "Pause Capture",
            Self::Resume => "Resume Capture",
            Self::OpenLibrary => "Open Library",
            Self::Settings => "Settings…",
            Self::ShowFloatingBar => "Show Floating Bar",
            Self::Quit => "Quit Encore",
        }
    }
}

/// The pure core of the tray menu: which actions appear, and in what order,
/// for a given (menu-bar mode, live capture state) pair. Free of any Tauri
/// type, so it is directly testable without a live app.
///
/// Off: the historical Show/Hide/Pause-or-Resume/Quit menu, unchanged.
/// On: the bar is hidden, so every bar action must stay reachable — Save
/// Replay, Pause/Resume, Open Library, Settings…, Show Floating Bar, Quit.
pub(crate) fn menu_actions(menu_bar_mode: bool, capture: CaptureState) -> Vec<TrayAction> {
    let pause_or_resume = if capture == CaptureState::Paused {
        TrayAction::Resume
    } else {
        TrayAction::Pause
    };
    if menu_bar_mode {
        vec![
            TrayAction::SaveReplay,
            pause_or_resume,
            TrayAction::OpenLibrary,
            TrayAction::Settings,
            TrayAction::ShowFloatingBar,
            TrayAction::Quit,
        ]
    } else {
        vec![
            TrayAction::Show,
            TrayAction::Hide,
            pause_or_resume,
            TrayAction::Quit,
        ]
    }
}

enum Entry {
    Item(MenuItem<Wry>),
    Separator(PredefinedMenuItem<Wry>),
}

/// Translates an action list into a real Tauri menu, inserting a separator
/// before `Quit` and after `Hide` to keep the same visual grouping the
/// original Show/Hide/Pause/Quit menu had.
fn build_menu(app: &AppHandle, actions: &[TrayAction]) -> tauri::Result<Menu<Wry>> {
    let mut entries = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        let previous = index.checked_sub(1).map(|previous| actions[previous]);
        let boundary = *action == TrayAction::Quit || previous == Some(TrayAction::Hide);
        if boundary {
            entries.push(Entry::Separator(PredefinedMenuItem::separator(app)?));
        }
        entries.push(Entry::Item(MenuItem::with_id(
            app,
            action.id(),
            action.label(),
            true,
            None::<&str>,
        )?));
    }
    let refs: Vec<&dyn IsMenuItem<Wry>> = entries
        .iter()
        .map(|entry| match entry {
            Entry::Item(item) => item as &dyn IsMenuItem<Wry>,
            Entry::Separator(separator) => separator as &dyn IsMenuItem<Wry>,
        })
        .collect();
    Menu::with_items(app, &refs)
}

/// Builds the initial tray icon at startup. The menu starts in the
/// non-menu-bar shape; `wire` (called once `CaptureService` exists)
/// immediately rebuilds it to match the persisted mode.
pub(crate) fn build(app: &mut tauri::App) -> tauri::Result<()> {
    let menu = build_menu(app.handle(), &menu_actions(false, CaptureState::Stopped))?;
    let mut tray = TrayIconBuilder::with_id("encore")
        .menu(&menu)
        .tooltip("Encore — local replay")
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let tray = tray.build(app)?;
    app.manage(TrayHandle(tray));
    Ok(())
}

/// Wires the tray to live capture state and honors the persisted menu-bar
/// mode at startup: hides the bar and expands the menu when it was left on
/// at the previous quit. Capture-state changes rebuild the menu lazily
/// (via the existing `capture-state-changed` event) rather than the tray
/// tracking pause/resume item handles itself, so the tray menu's shape and
/// its pause/resume label always come from the same `menu_actions` call.
pub(crate) fn wire(app: &AppHandle, capture: &CaptureService) {
    let mode = capture.menu_bar_mode();
    if mode {
        crate::desktop::hide_window(app);
    }
    rebuild(app, mode, capture.snapshot().capture);
    let app_handle = app.clone();
    let capture = capture.clone();
    app.listen(CAPTURE_STATE_CHANGED_EVENT, move |_event| {
        rebuild(
            &app_handle,
            capture.menu_bar_mode(),
            capture.snapshot().capture,
        );
    });
}

fn rebuild(app: &AppHandle, menu_bar_mode: bool, capture: CaptureState) {
    let Some(tray) = app.try_state::<TrayHandle>() else {
        return;
    };
    if let Ok(menu) = build_menu(app, &menu_actions(menu_bar_mode, capture)) {
        let _ = tray.0.set_menu(Some(menu));
    }
}

/// Applies, persists, and broadcasts a menu-bar-mode change, and updates the
/// bar's visibility and the tray menu shape to match — the single path both
/// the Settings toggle (`update_menu_bar_mode`) and the tray's own "Show
/// Floating Bar" item route through.
pub(crate) fn set_menu_bar_mode(
    app: &AppHandle,
    enabled: bool,
) -> Result<SettingsSnapshot, String> {
    let capture = app
        .try_state::<CaptureService>()
        .ok_or_else(|| "capture_unavailable".to_string())?;
    let snapshot = capture.set_menu_bar_mode(enabled)?;
    if enabled {
        crate::desktop::hide_window(app);
    } else {
        crate::desktop::show_window_without_focus(app);
    }
    rebuild(app, enabled, capture.snapshot().capture);
    let _ = app.emit("settings-changed", snapshot.clone());
    Ok(snapshot)
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show" => show_window(app),
        "hide" => crate::desktop::hide_window(app),
        "save_replay" => hotkeys::dispatch(app, HotkeyId::SaveReplay),
        "pause" | "resume" => hotkeys::dispatch(app, HotkeyId::PauseCapture),
        "open_library" => hotkeys::dispatch(app, HotkeyId::OpenLibrary),
        "settings" => {
            let _ = crate::desktop::open_settings_window(app);
        }
        "show_floating_bar" => {
            let _ = set_menu_bar_mode(app, false);
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

fn show_window(app: &AppHandle) {
    crate::desktop::show_window_without_focus(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_mode_keeps_the_historical_show_hide_pause_quit_menu() {
        let actions = menu_actions(false, CaptureState::Capturing);
        assert_eq!(
            actions,
            vec![
                TrayAction::Show,
                TrayAction::Hide,
                TrayAction::Pause,
                TrayAction::Quit,
            ]
        );
    }

    #[test]
    fn off_mode_shows_resume_when_capture_is_paused() {
        let actions = menu_actions(false, CaptureState::Paused);
        assert!(actions.contains(&TrayAction::Resume));
        assert!(!actions.contains(&TrayAction::Pause));
    }

    #[test]
    fn on_mode_exposes_every_bar_action_while_the_bar_is_hidden() {
        let actions = menu_actions(true, CaptureState::Capturing);
        assert_eq!(
            actions,
            vec![
                TrayAction::SaveReplay,
                TrayAction::Pause,
                TrayAction::OpenLibrary,
                TrayAction::Settings,
                TrayAction::ShowFloatingBar,
                TrayAction::Quit,
            ]
        );
    }

    #[test]
    fn on_mode_shows_resume_when_capture_is_paused() {
        let actions = menu_actions(true, CaptureState::Paused);
        assert!(actions.contains(&TrayAction::Resume));
        assert!(!actions.contains(&TrayAction::Pause));
    }

    #[test]
    fn every_action_has_a_stable_menu_id_distinct_from_the_others() {
        let all = [
            TrayAction::Show,
            TrayAction::Hide,
            TrayAction::SaveReplay,
            TrayAction::Pause,
            TrayAction::Resume,
            TrayAction::OpenLibrary,
            TrayAction::Settings,
            TrayAction::ShowFloatingBar,
            TrayAction::Quit,
        ];
        let mut ids: Vec<&str> = all.iter().map(|action| action.id()).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), before);
    }
}
