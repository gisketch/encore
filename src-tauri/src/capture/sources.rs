use super::model::{CaptureSource, SourceKind};
use screencapturekit::prelude::*;
use std::process;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGMainDisplayID() -> u32;
}

pub fn list_sources() -> Result<Vec<CaptureSource>, &'static str> {
    let content = SCShareableContent::create()
        .with_on_screen_windows_only(true)
        .with_exclude_desktop_windows(true)
        .get()
        .map_err(|_| "shareable_content_unavailable")?;
    let main_id = unsafe { CGMainDisplayID() };
    let mut sources = Vec::new();

    for (index, display) in content.displays().into_iter().enumerate() {
        sources.push(CaptureSource {
            id: format!("display:{}", display.display_id()),
            kind: SourceKind::Display,
            label: if display.display_id() == main_id {
                "Main display".into()
            } else {
                format!("Display {}", index + 1)
            },
            width: display.width(),
            height: display.height(),
            is_main: display.display_id() == main_id,
            bundle_id: None,
            title: None,
        });
    }

    for window in content.windows() {
        let Some(application) = window.owning_application() else {
            continue;
        };
        if application.process_id() as u32 == process::id() || !window.is_on_screen() {
            continue;
        }
        let frame = window.frame();
        let title = window.title().unwrap_or_default();
        if title.trim().is_empty() || frame.size.width < 2.0 || frame.size.height < 2.0 {
            continue;
        }
        sources.push(CaptureSource {
            id: format!("window:{}", window.window_id()),
            kind: SourceKind::Window,
            label: format!("{} — {}", application.application_name(), title),
            width: frame.size.width.round() as u32,
            height: frame.size.height.round() as u32,
            is_main: false,
            bundle_id: Some(application.bundle_identifier()),
            title: Some(title),
        });
    }
    Ok(sources)
}

pub fn main_display_source() -> Result<CaptureSource, &'static str> {
    list_sources()?
        .into_iter()
        .find(|source| source.kind == SourceKind::Display && source.is_main)
        .ok_or("main_display_unavailable")
}

pub fn source_exists(source_id: &str) -> bool {
    list_sources()
        .map(|sources| sources.iter().any(|source| source.id == source_id))
        .unwrap_or(false)
}
