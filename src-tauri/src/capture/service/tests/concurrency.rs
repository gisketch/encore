use super::*;
use std::{sync::Barrier, thread, time::Duration};

struct ConcurrencyProbeBackend {
    active_calls: Arc<AtomicUsize>,
    overlap_detected: Arc<AtomicBool>,
}

impl ConcurrencyProbeBackend {
    fn probe(&self) {
        if self.active_calls.fetch_add(1, Ordering::SeqCst) > 0 {
            self.overlap_detected.store(true, Ordering::SeqCst);
        }
        thread::sleep(Duration::from_millis(50));
        self.active_calls.fetch_sub(1, Ordering::SeqCst);
    }
}

impl CaptureBackend for ConcurrencyProbeBackend {
    fn list_sources(&self) -> Result<Vec<CaptureSource>, &'static str> {
        self.probe();
        Ok(vec![source("display:1", SourceKind::Display, true)])
    }

    fn main_display_source(&self) -> Result<CaptureSource, &'static str> {
        self.probe();
        Ok(source("display:1", SourceKind::Display, true))
    }

    fn source_exists(&self, source_id: &str) -> bool {
        source_id == "display:1"
    }

    fn start(
        &self,
        source: &CaptureSource,
        _frames: LatestSender<NativeFrame>,
        ready: Sender<CaptureReady>,
        _signals: Sender<RuntimeSignal>,
    ) -> Result<Box<dyn RunningCapture>, &'static str> {
        self.probe();
        let _ = ready.send(CaptureReady {
            source_id: source.id.clone(),
            width: source.width,
            height: source.height,
            status: SCFrameStatus::Complete,
        });
        Ok(Box::new(FakeStream {
            resize_count: Arc::new(AtomicUsize::new(0)),
        }))
    }
}

#[test]
fn source_enumeration_does_not_overlap_capture_startup() {
    let active_calls = Arc::new(AtomicUsize::new(0));
    let overlap_detected = Arc::new(AtomicBool::new(false));
    let service = service_with_backend(Box::new(ConcurrencyProbeBackend {
        active_calls,
        overlap_detected: overlap_detected.clone(),
    }));
    let barrier = Arc::new(Barrier::new(3));

    let startup = service.clone();
    let startup_barrier = barrier.clone();
    let startup_thread = thread::spawn(move || {
        startup_barrier.wait();
        startup.start_default().unwrap();
    });
    let listing = service.clone();
    let listing_barrier = barrier.clone();
    let listing_thread = thread::spawn(move || {
        listing_barrier.wait();
        listing.list_sources().unwrap();
    });
    barrier.wait();
    startup_thread.join().unwrap();
    listing_thread.join().unwrap();

    assert!(!overlap_detected.load(Ordering::SeqCst));
}
