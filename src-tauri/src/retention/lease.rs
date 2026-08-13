use super::{store::StoreInner, RetainedSegment};
use std::sync::{Mutex, Weak};

pub(crate) struct SegmentLease {
    pub(super) store: Weak<Mutex<StoreInner>>,
    pub(super) segments: Vec<RetainedSegment>,
}

impl Clone for SegmentLease {
    fn clone(&self) -> Self {
        if let Some(store) = self.store.upgrade() {
            let mut inner = store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for segment in &self.segments {
                *inner.leases.entry(segment.path.clone()).or_default() += 1;
            }
        }
        Self {
            store: self.store.clone(),
            segments: self.segments.clone(),
        }
    }
}

impl SegmentLease {
    pub(crate) fn segments(&self) -> &[RetainedSegment] {
        &self.segments
    }
}

impl Drop for SegmentLease {
    fn drop(&mut self) {
        let Some(store) = self.store.upgrade() else {
            return;
        };
        let mut inner = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for segment in &self.segments {
            let Some(count) = inner.leases.get_mut(&segment.path) else {
                continue;
            };
            *count -= 1;
            if *count == 0 {
                inner.leases.remove(&segment.path);
            }
        }
    }
}
