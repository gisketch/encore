use super::{timeline::Timeline, writer::VideoWriter};
use crate::capture::{NativeFrame, RuntimeSignal, SegmentBoundary, SegmentRecord};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

struct ActiveSegment {
    writer: VideoWriter,
    source_id: String,
    width: u32,
    height: u32,
    started_session_micros: u64,
    last_session_micros: u64,
    last_media_micros: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderCommand {
    Stop,
    Halt,
}

impl ActiveSegment {
    fn complete(self, boundary: SegmentBoundary) -> Result<SegmentRecord, &'static str> {
        let path = self.writer.finish()?;
        let byte_size = fs::metadata(&path)
            .map_err(|_| "segment_metadata_failed")?
            .len();
        Ok(SegmentRecord {
            path: path.to_string_lossy().into_owned(),
            source_id: self.source_id,
            width: self.width,
            height: self.height,
            byte_size,
            duration_micros: self.last_media_micros.max(0) as u64,
            session_start_micros: self.started_session_micros,
            session_end_micros: self.last_session_micros,
            completed_at_unix_ms: unix_millis(),
            boundary,
        })
    }
}

pub fn run(
    receiver: Receiver<NativeFrame>,
    controls: Receiver<EncoderCommand>,
    output_dir: PathBuf,
    signals: Sender<RuntimeSignal>,
) {
    if fs::create_dir_all(&output_dir).is_err() {
        send_failure(&signals, "segment_directory_unavailable");
        return;
    }
    let origin = Instant::now();
    let mut timeline = Timeline::default();
    let mut active: Option<ActiveSegment> = None;
    let mut sequence = 0_u64;
    let mut encoder_started = false;
    let mut halted = false;

    loop {
        match controls.try_recv() {
            Ok(EncoderCommand::Stop) => {
                finish_active(&mut active, SegmentBoundary::CaptureEnded, &signals);
                timeline = Timeline::default();
                encoder_started = false;
                let _ = signals.send(RuntimeSignal::EncoderStopped);
                continue;
            }
            Ok(EncoderCommand::Halt) => {
                finish_active(&mut active, SegmentBoundary::CaptureEnded, &signals);
                timeline = Timeline::default();
                encoder_started = false;
                halted = true;
                continue;
            }
            Err(TryRecvError::Disconnected | TryRecvError::Empty) => {}
        }
        let frame = match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(frame) => frame,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        };
        if halted {
            continue;
        }
        let Some(pixel_buffer) = frame.pixel_buffer() else {
            continue;
        };
        let encoded_geometry = (
            pixel_buffer.width().try_into().unwrap_or(u32::MAX),
            pixel_buffer.height().try_into().unwrap_or(u32::MAX),
        );
        let arrival_micros = frame
            .arrived_at
            .saturating_duration_since(origin)
            .as_micros()
            .try_into()
            .unwrap_or(u64::MAX);
        let stamp = timeline.observe(
            &frame.source_id,
            encoded_geometry,
            frame.pts_value,
            frame.pts_timescale,
            arrival_micros,
        );

        if let Some(boundary) = stamp.boundary {
            if boundary == SegmentBoundary::ClockDiscontinuity {
                // A multi-second timestamp jump with no stream error is the
                // signature of a sleep/wake gap; route it through the same
                // bounded recovery path as a stream error rather than only
                // cutting a new segment silently.
                let _ = signals.send(RuntimeSignal::ClockDiscontinuity(frame.source_id.clone()));
            }
            if let Some(segment) = active.take() {
                match segment.complete(boundary) {
                    Ok(record) => send_segment(&signals, record),
                    Err(code) => {
                        send_failure(&signals, code);
                        return;
                    }
                }
            }
        }

        if active.is_none() {
            sequence = sequence.saturating_add(1);
            match start_segment(
                &output_dir,
                sequence,
                &frame.source_id,
                encoded_geometry.0,
                encoded_geometry.1,
                timeline.segment_session_start(),
            ) {
                Ok(segment) => active = Some(segment),
                Err(code) => {
                    send_failure(&signals, code);
                    return;
                }
            }
        }

        let Some(segment) = active.as_mut() else {
            continue;
        };
        if let Err(code) = segment
            .writer
            .append(pixel_buffer.as_ptr(), stamp.media_micros)
        {
            send_failure(&signals, code);
            return;
        }
        segment.last_media_micros = stamp.media_micros;
        segment.last_session_micros = stamp.session_micros;
        if !encoder_started {
            encoder_started = signals.send(RuntimeSignal::EncoderStarted).is_ok();
        }
    }

    finish_active(&mut active, SegmentBoundary::CaptureEnded, &signals);
}

fn start_segment(
    output_dir: &Path,
    sequence: u64,
    source_id: &str,
    width: u32,
    height: u32,
    session_micros: u64,
) -> Result<ActiveSegment, &'static str> {
    let path = output_dir.join(format!("segment-{}-{sequence:06}.mp4", unix_millis()));
    let writer = VideoWriter::create(&path, width, height)?;
    Ok(ActiveSegment {
        writer,
        source_id: source_id.to_owned(),
        width,
        height,
        started_session_micros: session_micros,
        last_session_micros: session_micros,
        last_media_micros: 0,
    })
}

fn send_segment(signals: &Sender<RuntimeSignal>, record: SegmentRecord) {
    let _ = signals.send(RuntimeSignal::SegmentComplete(record));
}

fn send_failure(signals: &Sender<RuntimeSignal>, code: &'static str) {
    let _ = signals.send(RuntimeSignal::EncoderFailed(code.to_owned()));
}

fn finish_active(
    active: &mut Option<ActiveSegment>,
    boundary: SegmentBoundary,
    signals: &Sender<RuntimeSignal>,
) {
    let Some(segment) = active.take() else {
        return;
    };
    match segment.complete(boundary) {
        Ok(record) => send_segment(signals, record),
        Err(code) => send_failure(signals, code),
    }
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
