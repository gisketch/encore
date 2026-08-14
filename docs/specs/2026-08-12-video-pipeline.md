# Video Pipeline and Segment Contract

> Status: **APPROVED** — internally grilled 2026-08-12.

## Problem and Outcome

Raw frames are too expensive to retain and timestamps can become unreliable
across load, sleep, and capture-rate changes. The desired outcome is a bounded
hardware-encoded stream of independently usable fragmented MP4 segments.

## In Scope

- VideoToolbox H.264 encoding for macOS capture frames.
- A monotonic timestamp/timebase contract suitable for future audio sync.
- One-second keyframes and approximately 10-second fragmented MP4 segments.
- Segment-complete events carrying duration, byte size, and time bounds.

## Out of Scope

- Retention policy, global shortcuts, final concatenation, and audio encoding.

## Acceptance Headlines

- On supported macOS hardware, accepted capture frames produce H.264 MP4 files
  near 1080p30 and 3 Mbps using a required hardware encoder.
- A completed segment is independently playable, starts near timestamp zero,
  contains ordered presentation timestamps, and is approximately 10 seconds
  long during continuous capture.
- Files are invisible to downstream consumers while incomplete. A successful
  finalize atomically replaces a `.partial.mp4` file with a completed `.mp4`.
- A source or encoded-size change closes the current segment and starts the next
  one. Clock regression or a gap over two seconds also creates a clean boundary
  rather than a frozen or invalid timeline.
- Encoder creation, frame append, and finalization failures move the pipeline
  into an observable failed state with a stable local error code. Capture may
  continue, but Encore must not claim that replay evidence is being retained.
- Segment-complete events include the local path, source identity, encoded
  geometry, byte size, duration, monotonic session bounds, and completion time.

## Settled Constraints

- Target approximately 3 Mbps H.264 with a one-second keyframe interval.
- ScreenCaptureKit presentation timestamps are the media clock. Valid PTS is
  converted to integer microseconds, forced strictly monotonic when duplicate,
  and normalized to zero at each segment boundary. Arrival time is used only
  when capture PTS is invalid; wall-clock time is metadata, not media time.
- Write completed segments to disk; do not accumulate the replay in RAM.
- The macOS adapter uses AVFoundation's asset writer over VideoToolbox and
  requires hardware acceleration. This preserves the native `CVPixelBuffer`
  path and avoids copying raw 1080p frames through an FFmpeg process.
- The writer requests one-second movie fragments, disables frame reordering,
  and requests a 30-frame maximum keyframe interval at the 30 fps target.
- No software fallback is allowed. Unsupported or overloaded hardware is an
  actionable pipeline failure.
- The pipeline owns encoding and segment completion only. Retention, startup
  cleanup of crash remnants, and deletion policy belong to the rolling-buffer
  specification.

## Expected Validation

- Deterministic tests cover duplicate, regressing, missing, gapped, and
  variably paced timestamps plus source and geometry boundaries.
- A macOS hardware smoke produces a real segment from synthetic pixel buffers.
- `ffprobe` media inspection confirms H.264, expected geometry, playable MP4,
  ordered timestamps, approximate bitrate, and keyframes no more than about one
  second apart.

## Risks

- AVFoundation may reject the required hardware encoder on unusual or heavily
  loaded Macs; Encore reports this instead of producing lower-confidence output.
- Fragment and keyframe cadence are requests to the platform encoder and must be
  verified on the supported hardware matrix.
- A crash can leave `.partial.mp4` files. Their startup treatment is deliberately
  deferred to the rolling-buffer and crash-recovery contract.

## Internal Grill Record

1. **Who owns muxing?** Native AVFoundation owns encoding and MP4 writing for
   macOS. It keeps `CVPixelBuffer` delivery zero-copy and removes an avoidable
   runtime sidecar from the first platform. A later cross-platform packager may
   still use bundled FFmpeg for final concatenation.
2. **What happens without hardware encoding?** The pipeline fails visibly. A
   software fallback would violate the steady-state performance promise.
3. **How are clock anomalies handled?** Small duplicate/regressing values are
   repaired monotonically; a regression or gap over two seconds starts a new
   segment and records a discontinuity boundary.
4. **What happens on source or resolution change?** Finish the current file and
   open a new writer sized to the next native pixel buffer.
5. **When is a file durable?** Only after writer finalization and atomic rename.
   Consumers never receive `.partial.mp4` paths.
