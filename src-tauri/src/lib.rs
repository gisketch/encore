mod capture;
mod desktop;
mod encoder;
mod packager;
mod replay;
mod retention;

use capture::{CaptureService, CaptureSnapshot, CaptureSource, DiagnosticRecord};
use replay::{ReplayService, ReplaySnapshot};
use tauri::Manager;

#[tauri::command]
fn capture_snapshot(service: tauri::State<'_, CaptureService>) -> CaptureSnapshot {
    service.snapshot()
}

#[tauri::command]
fn request_capture_permission(service: tauri::State<'_, CaptureService>) -> CaptureSnapshot {
    service.request_permission()
}

#[tauri::command]
fn list_capture_sources(
    service: tauri::State<'_, CaptureService>,
) -> Result<Vec<CaptureSource>, String> {
    service.list_sources()
}

#[tauri::command]
fn switch_capture_source(
    service: tauri::State<'_, CaptureService>,
    source_id: String,
) -> Result<CaptureSnapshot, String> {
    service.switch_by_id(&source_id)
}

#[tauri::command]
fn retry_capture(service: tauri::State<'_, CaptureService>) -> Result<CaptureSnapshot, String> {
    service.retry()
}

#[tauri::command]
fn start_capture(service: tauri::State<'_, CaptureService>) -> Result<CaptureSnapshot, String> {
    service.start_default()
}

#[tauri::command]
fn stop_capture(service: tauri::State<'_, CaptureService>) -> CaptureSnapshot {
    service.stop()
}

#[tauri::command]
fn pause_capture(service: tauri::State<'_, CaptureService>) -> Result<CaptureSnapshot, String> {
    service.pause()
}

#[tauri::command]
fn resume_capture(service: tauri::State<'_, CaptureService>) -> Result<CaptureSnapshot, String> {
    service.resume()
}

#[tauri::command]
fn capture_diagnostics(service: tauri::State<'_, CaptureService>) -> DiagnosticRecord {
    service.diagnostics()
}

#[tauri::command]
fn set_retention_minutes(
    service: tauri::State<'_, CaptureService>,
    minutes: u8,
) -> Result<CaptureSnapshot, String> {
    service.set_retention_minutes(minutes)
}

#[tauri::command]
fn replay_snapshot(service: tauri::State<'_, ReplayService>) -> ReplaySnapshot {
    service.snapshot()
}

#[tauri::command]
fn trigger_replay(
    app: tauri::AppHandle,
    service: tauri::State<'_, ReplayService>,
) -> Result<ReplaySnapshot, String> {
    replay::trigger_and_emit(&app, service.inner())
}

#[tauri::command]
fn retry_replay(
    app: tauri::AppHandle,
    service: tauri::State<'_, ReplayService>,
) -> Result<ReplaySnapshot, String> {
    replay::retry_and_emit(&app, service.inner())
}

#[tauri::command]
fn reveal_saved_replay(
    app: tauri::AppHandle,
    service: tauri::State<'_, ReplayService>,
    replay_id: String,
) -> Result<ReplaySnapshot, String> {
    replay::reveal_and_emit(&app, service.inner(), &replay_id)
}

#[tauri::command]
fn open_screen_recording_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn()
        .map(|_| ())
        .map_err(|_| "settings_open_failed".into())
}

#[tauri::command]
fn open_export_folder(app: tauri::AppHandle) -> Result<(), String> {
    let folder = app
        .path()
        .video_dir()
        .map_err(|_| "export_folder_unavailable".to_string())?
        .join("Encore");
    std::fs::create_dir_all(&folder).map_err(|_| "export_folder_unavailable".to_string())?;
    std::process::Command::new("open")
        .arg(&folder)
        .spawn()
        .map(|_| ())
        .map_err(|_| "export_folder_open_failed".into())
}

#[tauri::command]
fn quit_encore(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            desktop::setup(app)?;
            let settings_path = app
                .path()
                .app_config_dir()
                .map_err(std::io::Error::other)?
                .join("settings.json");
            let capture = CaptureService::new(app.handle().clone(), settings_path);
            let replay_destination = app
                .path()
                .video_dir()
                .map_err(std::io::Error::other)?
                .join("Encore");
            let replay =
                ReplayService::new(capture.rolling_store(), capture.clone(), replay_destination)
                    .map_err(std::io::Error::other)?;
            app.manage(replay.clone());
            replay::register_global_shortcut(app.handle(), &replay);
            desktop::wire_capture_menu(app.handle(), &capture);
            app.manage(capture);
            Ok(())
        })
        .on_window_event(desktop::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            capture_snapshot,
            request_capture_permission,
            list_capture_sources,
            switch_capture_source,
            retry_capture,
            start_capture,
            stop_capture,
            pause_capture,
            resume_capture,
            capture_diagnostics,
            set_retention_minutes,
            replay_snapshot,
            trigger_replay,
            retry_replay,
            reveal_saved_replay,
            open_screen_recording_settings,
            open_export_folder,
            quit_encore
        ])
        .run(tauri::generate_context!())
        .expect("error while running Encore");
}
