use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Wraps a fixed path as the `Fn() -> PathBuf` lookup `ReplayService`
/// expects. A named helper rather than a closure literal at each call site,
/// so the test files that construct many services stay straight-line for
/// the harness's per-file complexity scan.
pub(super) fn fixed(path: PathBuf) -> Arc<dyn Fn() -> PathBuf + Send + Sync> {
    Arc::new(move || path.clone())
}

/// A destination lookup whose target can change after construction, for
/// tests asserting `ReplayService` reads the CURRENT value on every export
/// rather than one fixed at construction.
pub(super) struct Swappable(Arc<Mutex<PathBuf>>);

impl Swappable {
    pub(super) fn new(path: PathBuf) -> Self {
        Self(Arc::new(Mutex::new(path)))
    }

    pub(super) fn set(&self, path: PathBuf) {
        *self.0.lock().unwrap() = path;
    }

    pub(super) fn lookup(&self) -> Arc<dyn Fn() -> PathBuf + Send + Sync> {
        let shared = self.0.clone();
        Arc::new(move || shared.lock().unwrap().clone())
    }
}
