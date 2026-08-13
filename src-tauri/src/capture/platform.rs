#[cfg(target_os = "macos")]
mod macos {
    use super::super::{
        mailbox::LatestSender,
        model::{aspect_fit, CaptureSource, SourceKind},
        sources, RuntimeSignal,
    };
    use crossbeam_channel::Sender;
    use screencapturekit::cm::SCFrameStatus;
    use screencapturekit::prelude::*;
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        time::Instant,
    };

    pub struct NativeFrame {
        sample: CMSampleBuffer,
        pub source_id: String,
        pub pts_value: i64,
        pub pts_timescale: i32,
        pub arrived_at: Instant,
    }

    impl NativeFrame {
        pub fn pixel_buffer(&self) -> Option<screencapturekit::CVPixelBuffer> {
            self.sample.image_buffer()
        }
    }

    pub struct CaptureReady {
        pub source_id: String,
        pub width: u32,
        pub height: u32,
        pub status: SCFrameStatus,
    }

    pub struct NativeStream {
        stream: SCStream,
        committed: Arc<AtomicBool>,
    }

    impl NativeStream {
        pub fn commit(&self) {
            self.committed.store(true, Ordering::Release);
        }

        pub fn resize(&self, width: u32, height: u32) -> Result<(), &'static str> {
            let (width, height) = aspect_fit(width, height);
            self.stream
                .update_configuration(&stream_configuration(width, height))
                .map_err(|_| "stream_resize_failed")
        }
    }

    impl Drop for NativeStream {
        fn drop(&mut self) {
            let _ = self.stream.stop_capture();
        }
    }

    struct StreamDelegate {
        signals: Sender<RuntimeSignal>,
        committed: Arc<AtomicBool>,
        source_id: String,
    }

    impl SCStreamDelegateTrait for StreamDelegate {
        fn stream_did_become_active(&self) {
            if self.committed.load(Ordering::Acquire) {
                let _ = self
                    .signals
                    .try_send(RuntimeSignal::CheckAvailability(self.source_id.clone()));
            }
        }

        fn stream_did_become_inactive(&self) {
            if self.committed.load(Ordering::Acquire) {
                let _ = self
                    .signals
                    .try_send(RuntimeSignal::SourceUnavailable(self.source_id.clone()));
            }
        }

        fn did_stop_with_error(&self, _error: SCError) {
            if self.committed.load(Ordering::Acquire) {
                let _ = self
                    .signals
                    .try_send(RuntimeSignal::TransientFailure(self.source_id.clone()));
            }
        }
    }

    pub use sources::{list_sources, main_display_source, source_exists};

    pub fn start_stream(
        source: &CaptureSource,
        frames: LatestSender<NativeFrame>,
        ready: Sender<CaptureReady>,
        signals: Sender<RuntimeSignal>,
    ) -> Result<NativeStream, &'static str> {
        let content = shareable_content()?;
        let filter = match source.kind {
            SourceKind::Display => {
                let id = numeric_id(&source.id, "display:")?;
                let display = content
                    .displays()
                    .into_iter()
                    .find(|display| display.display_id() == id)
                    .ok_or("display_unavailable")?;
                SCContentFilter::create()
                    .with_display(&display)
                    .with_excluding_windows(&[])
                    .build()
            }
            SourceKind::Window => {
                let id = numeric_id(&source.id, "window:")?;
                let window = content
                    .windows()
                    .into_iter()
                    .find(|window| window.window_id() == id && window.is_on_screen())
                    .ok_or("window_unavailable")?;
                SCContentFilter::create().with_window(&window).build()
            }
        };
        let (width, height) = aspect_fit(source.width, source.height);
        let committed = Arc::new(AtomicBool::new(false));
        let configuration = stream_configuration(width, height);
        let mut stream = SCStream::new_with_delegate(
            &filter,
            &configuration,
            StreamDelegate {
                signals: signals.clone(),
                committed: committed.clone(),
                source_id: source.id.clone(),
            },
        );
        let health_signals = signals.clone();
        let output_committed = committed.clone();
        let output_source_id = source.id.clone();
        let last_geometry = Arc::new(Mutex::new(None));
        let output_id = stream.add_output_handler(
            move |sample: CMSampleBuffer, output_type: SCStreamOutputType| {
                if output_type != SCStreamOutputType::Screen {
                    return;
                }
                let has_image_buffer = sample.image_buffer().is_some();
                let status = effective_frame_status(sample.frame_status(), has_image_buffer);
                let info = sample.frame_info().unwrap_or_default();
                let pts = sample.output_presentation_timestamp();
                let geometry = info.content_rect.map(|rect| {
                    (
                        rect.size.width.round() as u32,
                        rect.size.height.round() as u32,
                    )
                });
                let (frame_width, frame_height) = geometry.unwrap_or((width, height));
                if output_committed.load(Ordering::Acquire) {
                    let mut previous = last_geometry
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if previous.is_some_and(|value| value != (frame_width, frame_height)) {
                        let _ = health_signals.try_send(RuntimeSignal::GeometryChanged(
                            output_source_id.clone(),
                            frame_width,
                            frame_height,
                        ));
                    }
                    *previous = Some((frame_width, frame_height));
                }
                let frame = NativeFrame {
                    sample,
                    source_id: output_source_id.clone(),
                    pts_value: pts.value,
                    pts_timescale: pts.timescale,
                    arrived_at: Instant::now(),
                };
                if !output_committed.load(Ordering::Acquire) && status != SCFrameStatus::Stopped {
                    let _ = ready.try_send(CaptureReady {
                        source_id: output_source_id.clone(),
                        width: frame_width,
                        height: frame_height,
                        status,
                    });
                }
                match status {
                    SCFrameStatus::Complete => {
                        if output_committed.load(Ordering::Acquire) {
                            frames.send_latest(frame);
                        }
                        if output_committed.load(Ordering::Acquire) {
                            let _ = health_signals
                                .try_send(RuntimeSignal::Healthy(output_source_id.clone()));
                        }
                    }
                    SCFrameStatus::Started | SCFrameStatus::Idle => {
                        if output_committed.load(Ordering::Acquire) {
                            frames.send_latest(frame);
                        }
                        if output_committed.load(Ordering::Acquire) {
                            let _ = health_signals
                                .try_send(RuntimeSignal::Healthy(output_source_id.clone()));
                        }
                    }
                    SCFrameStatus::Blank | SCFrameStatus::Suspended => {
                        if output_committed.load(Ordering::Acquire) {
                            let _ = health_signals
                                .try_send(RuntimeSignal::Paused(output_source_id.clone()));
                        }
                    }
                    SCFrameStatus::Stopped => {
                        if output_committed.load(Ordering::Acquire) {
                            let _ = health_signals.try_send(RuntimeSignal::CheckAvailability(
                                output_source_id.clone(),
                            ));
                        }
                    }
                }
            },
            SCStreamOutputType::Screen,
        );
        if output_id.is_none() {
            return Err("stream_output_registration_failed");
        }
        stream.start_capture().map_err(|_| "stream_start_failed")?;
        Ok(NativeStream { stream, committed })
    }

    fn shareable_content() -> Result<SCShareableContent, &'static str> {
        SCShareableContent::create()
            .with_on_screen_windows_only(true)
            .with_exclude_desktop_windows(true)
            .get()
            .map_err(|_| "shareable_content_unavailable")
    }

    fn stream_configuration(width: u32, height: u32) -> SCStreamConfiguration {
        SCStreamConfiguration::new()
            .with_width(width)
            .with_height(height)
            .with_pixel_format(PixelFormat::BGRA)
            .with_queue_depth(3)
            .with_minimum_frame_interval(&CMTime::new(1, 30))
            .with_shows_cursor(true)
            .with_captures_audio(false)
    }

    fn numeric_id(value: &str, prefix: &str) -> Result<u32, &'static str> {
        value
            .strip_prefix(prefix)
            .ok_or("invalid_source_id")?
            .parse()
            .map_err(|_| "invalid_source_id")
    }

    fn effective_frame_status(
        reported: Option<SCFrameStatus>,
        has_image_buffer: bool,
    ) -> SCFrameStatus {
        reported.unwrap_or(if has_image_buffer {
            SCFrameStatus::Complete
        } else {
            SCFrameStatus::Blank
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn missing_status_with_pixels_is_complete() {
            assert_eq!(effective_frame_status(None, true), SCFrameStatus::Complete);
        }

        #[test]
        fn explicit_blank_status_is_preserved() {
            assert_eq!(
                effective_frame_status(Some(SCFrameStatus::Blank), true),
                SCFrameStatus::Blank
            );
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
compile_error!("Encore capture currently supports macOS only");
