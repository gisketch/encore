use super::{RetainedSegment, RetentionSnapshot, RetentionState, RetentionWindow, SegmentLease};
use crate::capture::SegmentRecord;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::UNIX_EPOCH,
};

#[derive(Clone)]
pub(crate) struct RollingStore(Arc<Mutex<StoreInner>>);

pub(super) struct StoreInner {
    directory: PathBuf,
    window: RetentionWindow,
    segments: Vec<RetainedSegment>,
    pub(super) leases: HashMap<PathBuf, usize>,
    error_code: Option<String>,
    available: bool,
}

impl RollingStore {
    pub(crate) fn open(directory: PathBuf, minutes: u8) -> Result<Self, String> {
        let window = RetentionWindow::try_from(minutes).map_err(str::to_owned)?;
        fs::create_dir_all(&directory)
            .map_err(|_| "retention_directory_unavailable".to_string())?;
        let directory = fs::canonicalize(directory)
            .map_err(|_| "retention_directory_unavailable".to_string())?;
        let segments = recover(&directory)?;
        let store = Self(Arc::new(Mutex::new(StoreInner {
            directory,
            window,
            segments,
            leases: HashMap::new(),
            error_code: None,
            available: true,
        })));
        store.prune()?;
        Ok(store)
    }

    pub(crate) fn failed(directory: PathBuf, minutes: u8, code: &str) -> Self {
        Self(Arc::new(Mutex::new(StoreInner {
            directory,
            window: RetentionWindow::try_from(minutes).unwrap_or(RetentionWindow::TenMinutes),
            segments: Vec::new(),
            leases: HashMap::new(),
            error_code: Some(code.to_owned()),
            available: false,
        })))
    }

    pub(crate) fn admit(&self, record: &SegmentRecord) -> Result<RetentionSnapshot, String> {
        let mut inner = self.lock();
        ensure_available(&inner)?;
        let segment = validate_record(&inner.directory, record).map_err(|code| {
            inner.error_code = Some(code.to_owned());
            inner.available = false;
            code.to_owned()
        })?;
        if !inner.segments.iter().any(|item| item.path == segment.path) {
            inner.segments.push(segment);
            inner.segments.sort_by(segment_order);
        }
        prune_locked(&mut inner).map_err(|code| {
            inner.error_code = Some(code.to_owned());
            inner.available = false;
            code.to_owned()
        })?;
        inner.error_code = None;
        Ok(snapshot(&inner))
    }

    pub(crate) fn set_minutes(&self, minutes: u8) -> Result<RetentionSnapshot, String> {
        let window = RetentionWindow::try_from(minutes).map_err(str::to_owned)?;
        let mut inner = self.lock();
        ensure_available(&inner)?;
        let previous = inner.window;
        inner.window = window;
        prune_locked(&mut inner).map_err(|code| {
            inner.window = previous;
            inner.error_code = Some(code.to_owned());
            inner.available = false;
            code.to_owned()
        })?;
        inner.error_code = None;
        Ok(snapshot(&inner))
    }

    pub(crate) fn snapshot(&self) -> RetentionSnapshot {
        snapshot(&self.lock())
    }

    pub(crate) fn lease(&self) -> Result<SegmentLease, String> {
        let mut inner = self.lock();
        ensure_available(&inner)?;
        let segments = inner.segments[retained_start_index(&inner)..].to_vec();
        for segment in &segments {
            *inner.leases.entry(segment.path.clone()).or_default() += 1;
        }
        Ok(SegmentLease {
            store: Arc::downgrade(&self.0),
            segments,
        })
    }

    pub(crate) fn prune(&self) -> Result<RetentionSnapshot, String> {
        let mut inner = self.lock();
        ensure_available(&inner)?;
        prune_locked(&mut inner).map_err(|code| {
            inner.error_code = Some(code.to_owned());
            inner.available = false;
            code.to_owned()
        })?;
        inner.error_code = None;
        Ok(snapshot(&inner))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, StoreInner> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[cfg(test)]
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        self.lock()
            .segments
            .iter()
            .map(|segment| segment.path.clone())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn empty_for_tests() -> Self {
        Self(Arc::new(Mutex::new(StoreInner {
            directory: PathBuf::new(),
            window: RetentionWindow::TenMinutes,
            segments: Vec::new(),
            leases: HashMap::new(),
            error_code: None,
            available: true,
        })))
    }
}

fn recover(directory: &Path) -> Result<Vec<RetainedSegment>, String> {
    let entries = fs::read_dir(directory).map_err(|_| "retention_scan_failed".to_string())?;
    let mut segments = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| "retention_scan_failed".to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_partial_segment_name(&name) {
            fs::remove_file(path).map_err(|_| "retention_cleanup_failed".to_string())?;
            continue;
        }
        if !is_segment_name(&name) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|_| "retention_scan_failed".to_string())?;
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() == 0 {
            fs::remove_file(path).map_err(|_| "retention_cleanup_failed".to_string())?;
            continue;
        }
        segments.push(RetainedSegment {
            sort_time_ms: segment_time(&path, &metadata),
            path,
            byte_size: metadata.len(),
            source_id: None,
            duration_ms: 10_000,
        });
    }
    segments.sort_by(segment_order);
    Ok(segments)
}

fn validate_record(
    directory: &Path,
    record: &SegmentRecord,
) -> Result<RetainedSegment, &'static str> {
    let path = fs::canonicalize(&record.path).map_err(|_| "retention_segment_invalid")?;
    if path.parent() != Some(directory)
        || !path
            .file_name()
            .is_some_and(|name| is_segment_name(&name.to_string_lossy()))
    {
        return Err("retention_segment_invalid");
    }
    let metadata = fs::metadata(&path).map_err(|_| "retention_segment_invalid")?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() != record.byte_size {
        return Err("retention_segment_invalid");
    }
    Ok(RetainedSegment {
        sort_time_ms: parse_segment_time(&path).unwrap_or(record.completed_at_unix_ms),
        path,
        byte_size: metadata.len(),
        source_id: Some(record.source_id.clone()),
        duration_ms: record.duration_micros.div_ceil(1_000),
    })
}

fn prune_locked(inner: &mut StoreInner) -> Result<(), &'static str> {
    let keep_from = retained_start_index(inner);
    let segments = std::mem::take(&mut inner.segments);
    let mut remaining = segments.into_iter().enumerate();
    let mut kept = Vec::new();
    while let Some((index, segment)) = remaining.next() {
        let pinned = inner.leases.contains_key(&segment.path);
        if index < keep_from && !pinned {
            if fs::remove_file(&segment.path).is_err() {
                kept.push(segment);
                kept.extend(remaining.map(|(_, segment)| segment));
                inner.segments = kept;
                return Err("retention_delete_failed");
            }
        } else {
            kept.push(segment);
        }
    }
    inner.segments = kept;
    Ok(())
}

fn retained_start_index(inner: &StoreInner) -> usize {
    let Some(newest) = inner.segments.last() else {
        return 0;
    };
    let newest_end = newest.sort_time_ms.saturating_add(newest.duration_ms);
    let cutoff = newest_end.saturating_sub(inner.window.milliseconds());
    let first_inside = inner
        .segments
        .iter()
        .position(|segment| segment.sort_time_ms >= cutoff)
        .unwrap_or(0);
    first_inside.checked_sub(1).map_or(first_inside, |prior| {
        let segment = &inner.segments[prior];
        if segment.sort_time_ms.saturating_add(segment.duration_ms) > cutoff {
            prior
        } else {
            first_inside
        }
    })
}

fn snapshot(inner: &StoreInner) -> RetentionSnapshot {
    RetentionSnapshot {
        state: if inner.error_code.is_some() {
            RetentionState::Failed
        } else {
            RetentionState::Ready
        },
        minutes: inner.window.minutes(),
        retained_segments: inner.segments.len().try_into().unwrap_or(u64::MAX),
        retained_bytes: inner.segments.iter().fold(0_u64, |total, segment| {
            total.saturating_add(segment.byte_size)
        }),
        error_code: inner.error_code.clone(),
    }
}

fn ensure_available(inner: &StoreInner) -> Result<(), String> {
    if inner.available {
        Ok(())
    } else {
        Err(inner
            .error_code
            .clone()
            .unwrap_or_else(|| "retention_directory_unavailable".into()))
    }
}

fn is_segment_name(name: &str) -> bool {
    name.starts_with("segment-") && name.ends_with(".mp4") && !name.ends_with(".partial.mp4")
}

fn is_partial_segment_name(name: &str) -> bool {
    name.starts_with("segment-") && name.ends_with(".partial.mp4")
}

fn parse_segment_time(path: &Path) -> Option<u64> {
    path.file_name()?
        .to_str()?
        .strip_prefix("segment-")?
        .split('-')
        .next()?
        .parse()
        .ok()
}

fn segment_time(path: &Path, metadata: &fs::Metadata) -> u64 {
    parse_segment_time(path).unwrap_or_else(|| {
        metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| duration.as_millis().try_into().ok())
            .unwrap_or(0)
    })
}

fn segment_order(left: &RetainedSegment, right: &RetainedSegment) -> std::cmp::Ordering {
    left.sort_time_ms
        .cmp(&right.sort_time_ms)
        .then_with(|| left.path.cmp(&right.path))
}
