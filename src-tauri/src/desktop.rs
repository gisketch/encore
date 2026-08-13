use crate::capture::{CaptureService, CaptureState};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Listener, Manager, PhysicalPosition, Wry,
};

const CAPTURE_STATE_CHANGED_EVENT: &str = "capture-state-changed";

/// Handles to the tray's pause/resume items, kept managed state so their
/// enabled state can be refreshed whenever capture transitions.
struct TrayCaptureItems {
    pause: MenuItem<Wry>,
    resume: MenuItem<Wry>,
}

pub fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Encore", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide Encore", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause Capture", false, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume Capture", false, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let capture_separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Encore", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &hide,
            &separator,
            &pause,
            &resume,
            &capture_separator,
            &quit,
        ],
    )?;
    let mut tray = TrayIconBuilder::with_id("encore")
        .menu(&menu)
        .tooltip("Encore — local replay")
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    tray.build(app)?;
    app.manage(TrayCaptureItems { pause, resume });

    if let Some(window) = app.get_webview_window("main") {
        position_floating_window(&window)?;
    }
    Ok(())
}

/// Keeps the tray's Pause/Resume items reflecting the live capture state:
/// enabled only when the corresponding action is currently valid.
pub fn wire_capture_menu(app: &tauri::AppHandle, capture: &CaptureService) {
    refresh_capture_menu(app, capture.snapshot().capture);
    let app_handle = app.clone();
    let capture = capture.clone();
    app.listen(CAPTURE_STATE_CHANGED_EVENT, move |_event| {
        refresh_capture_menu(&app_handle, capture.snapshot().capture);
    });
}

fn refresh_capture_menu(app: &tauri::AppHandle, capture: CaptureState) {
    let Some(items) = app.try_state::<TrayCaptureItems>() else {
        return;
    };
    let _ = items.pause.set_enabled(capture == CaptureState::Capturing);
    let _ = items.resume.set_enabled(capture == CaptureState::Paused);
}

pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show" => show_window(app),
        "hide" => hide_window(app),
        "pause" => pause_capture(app),
        "resume" => resume_capture(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn pause_capture(app: &tauri::AppHandle) {
    if let Some(capture) = app.try_state::<CaptureService>() {
        let _ = capture.pause();
    }
}

fn resume_capture(app: &tauri::AppHandle) {
    if let Some(capture) = app.try_state::<CaptureService>() {
        let _ = capture.resume();
    }
}

fn show_window(app: &tauri::AppHandle) {
    show_window_without_focus(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }
}

pub(crate) fn show_window_without_focus(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = position_floating_window(&window);
    }
}

fn hide_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Shows and focuses the settings window, which `tauri.conf.json` declares
/// hidden at startup so it never flashes before the bar does.
pub fn open_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings_window_unavailable".to_string())?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|_| "settings_window_open_failed".to_string())
}

fn position_floating_window(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()?.or(window.primary_monitor()?) else {
        return Ok(());
    };
    let work_area = monitor.work_area();
    let window_size = window.outer_size()?;
    let margin = (16.0 * monitor.scale_factor()).round() as i32;
    let x =
        work_area.position.x + (work_area.size.width as i32 - window_size.width as i32).max(0) / 2;
    let y =
        work_area.position.y + work_area.size.height as i32 - window_size.height as i32 - margin;
    window.set_position(PhysicalPosition::new(x, y))
}
