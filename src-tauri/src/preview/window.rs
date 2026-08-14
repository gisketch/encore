//! The post-save preview window: which replay it is showing, where it
//! sits, and how it is raised without ever taking keyboard focus.

use super::{placement, placement::Rect, PreviewPayload};
use std::{
    path::Path,
    sync::{Mutex, PoisonError},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

/// The replay the preview window is currently showing. A single hidden
/// window is reused for every save (mirroring `EditorContext`), so "which
/// replay is on screen" lives here rather than in the window — the window
/// reads it on mount through the `preview_context` command and then
/// follows the `preview-changed` event.
pub(crate) type PreviewContext = Mutex<Option<PreviewPayload>>;

const LABEL: &str = "preview";

/// Logical points of clearance from the work area's edges, the same
/// margin `desktop::position_floating_window` gives the floating bar.
const MARGIN_POINTS: f64 = 16.0;

/// Fallback bar height in logical points, matching the `main` window's
/// declared height in `tauri.conf.json`, used only when the bar window
/// cannot be measured (it is hidden in menu-bar mode).
const BAR_HEIGHT_POINTS: f64 = 84.0;

/// Shows the preview for saved bundle `id` (the bundle's folder name —
/// `SavedReplaySnapshot::display_name`, never its session `id`).
///
/// Building the payload first means a replay that cannot be described is
/// never put on screen at all. The window is positioned from Rust, so the
/// page needs no window-positioning permission, and it is only `show`n —
/// never `set_focus`ed — so the application under test keeps keyboard
/// focus, which is the whole point of this surface.
///
/// The event carries the payload so an already-visible preview swaps its
/// contents in place; a second save therefore reuses this one window
/// instead of opening another.
///
/// The asset protocol is granted `destination` exactly as `editor::open`
/// grants it, and for the same reason: the card's `<video>` loads the
/// replay file through `convertFileSrc`, which is refused without it — the
/// video would silently never load. The scope is process-wide (Tauri has
/// no per-window scope), so this only ever grants the resolved save
/// destination tree, and re-granting it on every save is a no-op.
pub(crate) fn show(app: &AppHandle, destination: &Path, id: &str) -> Result<(), String> {
    let payload = super::build(destination, id)?;
    let window = app
        .get_webview_window(LABEL)
        .ok_or_else(|| "preview_window_unavailable".to_string())?;
    app.asset_protocol_scope()
        .allow_directory(destination, true)
        .map_err(|_| "preview_scope_failed".to_string())?;
    if let Some(context) = app.try_state::<PreviewContext>() {
        set(context.inner(), Some(payload.clone()));
    }
    let _ = position(app, &window);
    window
        .show()
        .map_err(|_| "preview_window_show_failed".to_string())?;
    let _ = app.emit("preview-changed", payload);
    Ok(())
}

/// Records `payload` as the preview's current replay, replacing whatever
/// it was showing.
pub(crate) fn set(context: &PreviewContext, payload: Option<PreviewPayload>) {
    *context.lock().unwrap_or_else(PoisonError::into_inner) = payload;
}

/// The replay the preview is currently showing, if any — the window's own
/// bootstrap read.
pub(crate) fn current(context: &PreviewContext) -> Option<PreviewPayload> {
    context
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Moves the window into the work area's bottom-right corner, clear of
/// the floating bar. Failure is not fatal to `show`: an unplaced preview
/// is still better feedback than none.
fn position(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()?.or(window.primary_monitor()?) else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let work_area = monitor.work_area();
    let size = window.outer_size()?;
    let margin = (MARGIN_POINTS * scale).round() as i32;
    let (x, y) = placement::bottom_right(
        Rect {
            x: work_area.position.x,
            y: work_area.position.y,
            width: work_area.size.width as i32,
            height: work_area.size.height as i32,
        },
        (size.width as i32, size.height as i32),
        margin,
        reserved_bottom(app, scale, margin),
    );
    window.set_position(PhysicalPosition::new(x, y))
}

/// The bottom band the floating bar occupies: its measured height plus
/// its own margin. Measured rather than assumed so the two windows cannot
/// drift apart if the bar is resized (it expands for the advanced row).
fn reserved_bottom(app: &AppHandle, scale: f64, margin: i32) -> i32 {
    let measured = app
        .get_webview_window("main")
        .and_then(|bar| bar.outer_size().ok())
        .map(|size| size.height as i32)
        .unwrap_or_else(|| (BAR_HEIGHT_POINTS * scale).round() as i32);
    measured + margin
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(id: &str) -> PreviewPayload {
        PreviewPayload {
            id: id.to_string(),
            display_name: id.to_string(),
            duration_seconds: Some(12),
            total_bytes: 4096,
            video_path: format!("/tmp/{id}/replay.mp4"),
        }
    }

    #[test]
    fn a_fresh_context_is_showing_nothing() {
        assert_eq!(current(&PreviewContext::new(None)), None);
    }

    #[test]
    fn a_second_save_replaces_the_shown_replay_instead_of_stacking() {
        let context = PreviewContext::new(None);
        set(&context, Some(payload("2026-08-14_16-32-05")));
        set(&context, Some(payload("2026-08-14_16-32-11")));
        assert_eq!(
            current(&context).map(|shown| shown.id),
            Some("2026-08-14_16-32-11".to_string())
        );
    }

    #[test]
    fn dismissing_clears_what_the_window_reads_on_its_next_mount() {
        let context = PreviewContext::new(None);
        set(&context, Some(payload("2026-08-14_16-32-05")));
        set(&context, None);
        assert_eq!(current(&context), None);
    }
}
