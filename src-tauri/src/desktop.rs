mod tray;
mod tray_click;
mod tray_status;

use crate::capture::CaptureService;
use tauri::{Manager, PhysicalPosition};

pub fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    tray::build(app)?;
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    if let Some(window) = app.get_webview_window("main") {
        position_floating_window(&window)?;
    }
    Ok(())
}

/// Wires the tray to live capture state (see `tray::wire`).
pub fn wire_capture_menu(app: &tauri::AppHandle, capture: &CaptureService) {
    tray::wire(app, capture);
}

pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

pub(crate) fn show_window_without_focus(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = position_floating_window(&window);
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

/// Shows and focuses the Library window, mirroring
/// `open_settings_window` — `tauri.conf.json` declares it hidden at
/// startup so it never flashes before the bar does.
pub fn open_library_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("library")
        .ok_or_else(|| "library_window_unavailable".to_string())?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|_| "library_window_open_failed".to_string())
}

/// Shows and focuses the Editor window, mirroring `open_library_window`.
/// Callers validate the requested replay and record it in `EditorContext`
/// before calling this — this function only handles window visibility.
pub fn open_editor_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("editor")
        .ok_or_else(|| "editor_window_unavailable".to_string())?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|_| "editor_window_open_failed".to_string())
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
