use super::CaptureService;
use crate::capture::model::CaptureSnapshot;
use crate::capture::settings::{self, PersistedTarget, SettingsDocument};

/// Starts capture into the persisted target, resolving a window by
/// best-effort identity and falling back to the default display with a
/// visible notice when it cannot be found.
pub(super) fn start_from_persisted(service: &CaptureService) -> Result<CaptureSnapshot, String> {
    let target = service.default_target();
    let resolved = service
        .list_sources()
        .ok()
        .and_then(|sources| settings::resolve_target(&sources, &target));
    match resolved {
        Some(source) => service.switch_source(source),
        None => {
            service.start_default()?;
            if matches!(target, PersistedTarget::Window { .. }) {
                service.update(|snapshot| {
                    snapshot.source_notice = Some("persisted_window_unavailable".into());
                });
            }
            Ok(service.snapshot())
        }
    }
}

pub(super) fn persist(service: &CaptureService) {
    let snapshot = service.snapshot();
    let target = snapshot
        .source
        .as_ref()
        .map(PersistedTarget::from_source)
        .unwrap_or_default();
    service.sync_default_target(target.clone());
    let document = SettingsDocument::new(
        snapshot.retention.minutes,
        target,
        service.appearance(),
        service.save_destination(),
        service.after_save(),
        service.hotkeys(),
        service.save_sound(),
    );
    let _ = service.0.settings.save(&document);
}
