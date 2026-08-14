mod capture;
mod desktop;
mod diagnostics;
mod editor;
mod encoder;
mod hotkeys;
mod library;
mod packager;
mod preview;
mod replay;
mod retention;
mod sound;

use capture::{CaptureService, CaptureSnapshot, CaptureSource, DiagnosticRecord, SettingsSnapshot};
use diagnostics::DiagnosticLog;
use editor::commands::{
    copy_export_to_clipboard, editor_context, editor_header, editor_keyframes, open_editor_window,
};
use editor::export::commands::{export_gif_replay, export_trimmed_replay};
use library::commands::{
    delete_replay, library_index, library_thumbnail, open_library_window, open_replay_file,
    reveal_replay_bundle,
};
use preview::commands::{preview_context, preview_payload};
use replay::{ReplayService, ReplaySnapshot};
use tauri::{Emitter, Manager};

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
fn open_export_folder(service: tauri::State<'_, CaptureService>) -> Result<(), String> {
    service.reveal_export_folder()
}

#[tauri::command]
fn quit_encore(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    desktop::open_settings_window(&app)
}

#[tauri::command]
fn settings_snapshot(service: tauri::State<'_, CaptureService>) -> SettingsSnapshot {
    service.settings_snapshot()
}

#[tauri::command]
fn update_appearance(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    appearance: String,
) -> Result<SettingsSnapshot, String> {
    service.set_appearance(appearance)?;
    Ok(broadcast_settings(&app, service.inner()))
}

#[tauri::command]
fn update_default_source(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    source_id: String,
) -> Result<SettingsSnapshot, String> {
    service.set_default_source_by_id(&source_id)?;
    Ok(broadcast_settings(&app, service.inner()))
}

#[tauri::command]
fn update_save_destination(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    path: String,
) -> Result<SettingsSnapshot, String> {
    service.set_save_destination(Some(std::path::PathBuf::from(path)))?;
    Ok(broadcast_settings(&app, service.inner()))
}

#[tauri::command]
fn update_after_save(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    after_save: String,
) -> Result<SettingsSnapshot, String> {
    service.set_after_save(after_save)?;
    Ok(broadcast_settings(&app, service.inner()))
}

#[tauri::command]
fn update_hotkey(
    app: tauri::AppHandle,
    capture: tauri::State<'_, CaptureService>,
    replay: tauri::State<'_, ReplayService>,
    id: String,
    accelerator: String,
) -> Result<SettingsSnapshot, String> {
    let hotkey_id = capture::HotkeyId::parse(&id)?;
    hotkeys::update_hotkey(
        &hotkeys::AppHandleRegistrar(&app),
        &app,
        capture.inner(),
        replay.inner(),
        hotkey_id,
        accelerator,
    )?;
    Ok(broadcast_settings(&app, capture.inner()))
}

/// Applies, persists, and broadcasts a menu-bar-mode change: hides/shows
/// the floating bar and reshapes the tray menu to match, via the same path
/// the tray's own "Show Floating Bar" item uses.
#[tauri::command]
fn update_menu_bar_mode(app: tauri::AppHandle, enabled: bool) -> Result<SettingsSnapshot, String> {
    desktop::set_menu_bar_mode(&app, enabled)
}

/// Applies, persists, and broadcasts the save-confirmation-sound toggle.
#[tauri::command]
fn update_save_sound(
    app: tauri::AppHandle,
    service: tauri::State<'_, CaptureService>,
    enabled: bool,
) -> Result<SettingsSnapshot, String> {
    service.set_save_sound(enabled)?;
    Ok(broadcast_settings(&app, service.inner()))
}

/// Reads the current settings snapshot and broadcasts it to every window,
/// so commands that change a single field don't each have to remember to.
fn broadcast_settings(app: &tauri::AppHandle, service: &CaptureService) -> SettingsSnapshot {
    let snapshot = service.settings_snapshot();
    let _ = app.emit("settings-changed", snapshot.clone());
    snapshot
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            desktop::setup(app)?;
            let settings_path = app
                .path()
                .app_config_dir()
                .map_err(std::io::Error::other)?
                .join("settings.json");
            let diagnostics_path = app
                .path()
                .app_log_dir()
                .map_err(std::io::Error::other)?
                .join("diagnostics.jsonl");
            let diagnostics = DiagnosticLog::open(diagnostics_path);
            let capture =
                CaptureService::new(app.handle().clone(), settings_path, diagnostics.clone());
            let replay = ReplayService::new(
                capture.rolling_store(),
                capture.clone(),
                capture.destination_lookup(),
                diagnostics,
            )
            .map_err(std::io::Error::other)?;
            app.manage(replay.clone());
            hotkeys::register_all(
                &hotkeys::AppHandleRegistrar(app.handle()),
                &capture,
                &replay,
            );
            desktop::wire_capture_menu(app.handle(), &capture);
            app.manage(capture);
            app.manage(editor::EditorContext::new(None));
            app.manage(preview::PreviewContext::new(None));
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
            quit_encore,
            open_settings_window,
            open_library_window,
            library_index,
            open_replay_file,
            delete_replay,
            library_thumbnail,
            reveal_replay_bundle,
            preview_payload,
            preview_context,
            open_editor_window,
            editor_context,
            editor_header,
            editor_keyframes,
            export_trimmed_replay,
            export_gif_replay,
            copy_export_to_clipboard,
            settings_snapshot,
            update_appearance,
            update_default_source,
            update_save_destination,
            update_after_save,
            update_hotkey,
            update_menu_bar_mode,
            update_save_sound
        ])
        .run(tauri::generate_context!())
        .expect("error while running Encore");
}
