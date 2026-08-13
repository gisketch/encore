use super::{
    mailbox::LatestSender,
    model::CaptureSource,
    platform::{self, CaptureReady, NativeFrame, NativeStream},
    RuntimeSignal,
};
use crossbeam_channel::Sender;

pub trait RunningCapture: Send {
    fn commit(&self);
    fn resize(&self, width: u32, height: u32) -> Result<(), &'static str>;
}

impl RunningCapture for NativeStream {
    fn commit(&self) {
        NativeStream::commit(self);
    }

    fn resize(&self, width: u32, height: u32) -> Result<(), &'static str> {
        NativeStream::resize(self, width, height)
    }
}

pub trait CaptureBackend: Send + Sync {
    fn list_sources(&self) -> Result<Vec<CaptureSource>, &'static str>;
    fn main_display_source(&self) -> Result<CaptureSource, &'static str>;
    fn source_exists(&self, source_id: &str) -> bool;
    fn start(
        &self,
        source: &CaptureSource,
        frames: LatestSender<NativeFrame>,
        ready: Sender<CaptureReady>,
        signals: Sender<RuntimeSignal>,
    ) -> Result<Box<dyn RunningCapture>, &'static str>;
}

pub struct SystemCaptureBackend;

impl CaptureBackend for SystemCaptureBackend {
    fn list_sources(&self) -> Result<Vec<CaptureSource>, &'static str> {
        platform::list_sources()
    }

    fn main_display_source(&self) -> Result<CaptureSource, &'static str> {
        platform::main_display_source()
    }

    fn source_exists(&self, source_id: &str) -> bool {
        platform::source_exists(source_id)
    }

    fn start(
        &self,
        source: &CaptureSource,
        frames: LatestSender<NativeFrame>,
        ready: Sender<CaptureReady>,
        signals: Sender<RuntimeSignal>,
    ) -> Result<Box<dyn RunningCapture>, &'static str> {
        platform::start_stream(source, frames, ready, signals)
            .map(|stream| Box::new(stream) as Box<dyn RunningCapture>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::model::SourceKind;

    struct FakeBackend {
        sources: Vec<CaptureSource>,
    }

    impl CaptureBackend for FakeBackend {
        fn list_sources(&self) -> Result<Vec<CaptureSource>, &'static str> {
            Ok(self.sources.clone())
        }

        fn main_display_source(&self) -> Result<CaptureSource, &'static str> {
            self.sources
                .iter()
                .find(|source| source.is_main)
                .cloned()
                .ok_or("main_display_unavailable")
        }

        fn source_exists(&self, source_id: &str) -> bool {
            self.sources.iter().any(|source| source.id == source_id)
        }

        fn start(
            &self,
            _source: &CaptureSource,
            _frames: LatestSender<NativeFrame>,
            _ready: Sender<CaptureReady>,
            _signals: Sender<RuntimeSignal>,
        ) -> Result<Box<dyn RunningCapture>, &'static str> {
            Err("fake_start_failure")
        }
    }

    #[test]
    fn fake_backend_controls_source_availability_without_screen_access() {
        let main = CaptureSource {
            id: "display:1".into(),
            kind: SourceKind::Display,
            label: "Main display".into(),
            width: 1920,
            height: 1080,
            is_main: true,
            bundle_id: None,
            title: None,
        };
        let backend = FakeBackend {
            sources: vec![main.clone()],
        };
        assert_eq!(backend.main_display_source().unwrap(), main);
        assert!(backend.source_exists("display:1"));
        assert!(!backend.source_exists("window:9"));
    }
}
