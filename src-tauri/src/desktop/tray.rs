use crate::capture::{CaptureService, CaptureState, HotkeyId};
use crate::hotkeys;
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Listener, Manager, Wry,
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
    SaveReplay,
    Pause,
    Resume,
    OpenLibrary,
    Settings,
    ShowActionBar,
    Quit,
}

impl TrayAction {
    fn id(self) -> &'static str {
        match self {
            Self::SaveReplay => "save_replay",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::OpenLibrary => "open_library",
            Self::Settings => "settings",
            Self::ShowActionBar => "show_action_bar",
            Self::Quit => "quit",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::SaveReplay => "Save Replay",
            Self::Pause => "Pause Capture",
            Self::Resume => "Resume Capture",
            Self::OpenLibrary => "Open Library",
            Self::Settings => "Settings…",
            Self::ShowActionBar => "Show Action Bar",
            Self::Quit => "Quit Encore",
        }
    }
}

/// The pure core of the tray menu: which actions appear, and in what order,
/// for a given live capture state. Free of any Tauri type, so it is
/// directly testable without a live app.
///
/// The menu bar is Encore's permanent control surface, not a mode: the same
/// complete list appears whether or not the floating action bar happens to
/// be visible, so a tester never has to know which surface they are in to
/// know where a control lives. Only the pause/resume item varies, and only
/// with the capture state it reports.
pub(crate) fn menu_actions(capture: CaptureState) -> Vec<TrayAction> {
    let pause_or_resume = if capture == CaptureState::Paused {
        TrayAction::Resume
    } else {
        TrayAction::Pause
    };
    vec![
        TrayAction::SaveReplay,
        pause_or_resume,
        TrayAction::OpenLibrary,
        TrayAction::Settings,
        TrayAction::ShowActionBar,
        TrayAction::Quit,
    ]
}

enum Entry {
    Item(MenuItem<Wry>),
    Separator(PredefinedMenuItem<Wry>),
}

/// Translates an action list into a real Tauri menu, inserting a separator
/// before `ShowActionBar` and before `Quit` so the capture actions, the
/// window action, and quitting read as three groups.
fn build_menu(app: &AppHandle, actions: &[TrayAction]) -> tauri::Result<Menu<Wry>> {
    let mut entries = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        let _ = index;
        let boundary = [TrayAction::Quit, TrayAction::ShowActionBar].contains(action) && index > 0;
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
    let menu = build_menu(app.handle(), &menu_actions(CaptureState::Stopped))?;
    let mut tray = TrayIconBuilder::with_id("encore")
        .menu(&menu)
        .tooltip("Encore — local replay")
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(super::tray_click::handle);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let tray = tray.build(app)?;
    app.manage(TrayHandle(tray));
    Ok(())
}

/// Wires the tray to live capture state. Capture-state changes rebuild the
/// menu lazily
/// (via the existing `capture-state-changed` event) rather than the tray
/// tracking pause/resume item handles itself, so the tray menu's shape and
/// its pause/resume label always come from the same `menu_actions` call.
pub(crate) fn wire(app: &AppHandle, capture: &CaptureService) {
    rebuild(app, capture.snapshot().capture);
    let app_handle = app.clone();
    let capture = capture.clone();
    app.listen(CAPTURE_STATE_CHANGED_EVENT, move |_event| {
        rebuild(&app_handle, capture.snapshot().capture);
    });
}

fn rebuild(app: &AppHandle, capture: CaptureState) {
    let Some(tray) = app.try_state::<TrayHandle>() else {
        return;
    };
    if let Ok(menu) = build_menu(app, &menu_actions(capture)) {
        let _ = tray.0.set_menu(Some(menu));
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "save_replay" => hotkeys::dispatch(app, HotkeyId::SaveReplay),
        "pause" | "resume" => hotkeys::dispatch(app, HotkeyId::PauseCapture),
        "open_library" => hotkeys::dispatch(app, HotkeyId::OpenLibrary),
        "settings" => {
            let _ = crate::desktop::open_settings_window(app);
        }
        "show_action_bar" => show_action_bar(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

/// Brings the floating action bar back and focuses it. The one restore
/// path, shared by the menu item and (MB-03) the icon's double-click: the
/// bar is only ever hidden, never destroyed, so this is always a show.
pub(crate) fn show_action_bar(app: &AppHandle) {
    crate::desktop::show_window_without_focus(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menu bar is the permanent control surface, so the menu must be
    /// complete on its own — this is the guarantee that a tester with the
    /// action bar hidden can still reach everything.
    #[test]
    fn the_menu_always_carries_every_action() {
        let actions = menu_actions(CaptureState::Capturing);
        assert_eq!(
            actions,
            vec![
                TrayAction::SaveReplay,
                TrayAction::Pause,
                TrayAction::OpenLibrary,
                TrayAction::Settings,
                TrayAction::ShowActionBar,
                TrayAction::Quit,
            ]
        );
    }

    /// Only the pause/resume item varies, and only with capture state.
    #[test]
    fn the_menu_shows_resume_when_capture_is_paused() {
        let actions = menu_actions(CaptureState::Paused);
        assert!(actions.contains(&TrayAction::Resume));
        assert!(!actions.contains(&TrayAction::Pause));
    }

    /// No capture state may drop an action: the menu's completeness cannot
    /// depend on what the pipeline happens to be doing.
    #[test]
    fn no_capture_state_can_shrink_the_menu() {
        for capture in [
            CaptureState::Stopped,
            CaptureState::Starting,
            CaptureState::Capturing,
            CaptureState::Paused,
            CaptureState::Recovering,
            CaptureState::SourceUnavailable,
            CaptureState::Failed,
        ] {
            let actions = menu_actions(capture);
            assert_eq!(actions.len(), 6, "{capture:?} changed the menu length");
            for required in [
                TrayAction::SaveReplay,
                TrayAction::OpenLibrary,
                TrayAction::Settings,
                TrayAction::ShowActionBar,
                TrayAction::Quit,
            ] {
                assert!(
                    actions.contains(&required),
                    "{capture:?} dropped {required:?}"
                );
            }
        }
    }

    #[test]
    fn every_action_has_a_stable_menu_id_distinct_from_the_others() {
        let all = [
            TrayAction::SaveReplay,
            TrayAction::Pause,
            TrayAction::Resume,
            TrayAction::OpenLibrary,
            TrayAction::Settings,
            TrayAction::ShowActionBar,
            TrayAction::Quit,
        ];
        let mut ids: Vec<&str> = all.iter().map(|action| action.id()).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), before);
    }
}
