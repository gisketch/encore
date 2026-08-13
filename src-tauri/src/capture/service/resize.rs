use super::CaptureService;

pub(super) fn handle(service: &CaptureService, source_id: &str, width: u32, height: u32) {
    let _operation = service
        .0
        .operation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !service.is_current_source(source_id) {
        return;
    }

    let Some(mut source) = service.snapshot().source else {
        return;
    };
    if (source.width, source.height) == (width, height) {
        return;
    }

    let resize_result = service
        .0
        .stream
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .ok_or("stream_unavailable")
        .and_then(|stream| stream.resize(width, height));

    source.width = width;
    source.height = height;
    if resize_result.is_ok() {
        service.update(|snapshot| snapshot.source = Some(source));
        return;
    }

    service.update(|snapshot| {
        snapshot.evidence_gap_count += 1;
        snapshot.error_code = Some("stream_resize_restart".into());
    });
    let _ = service.switch_source_locked(source);
}
