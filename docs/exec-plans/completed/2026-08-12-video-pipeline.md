# macOS Video Pipeline

Status: **COMPLETE** — VP-01 through VP-03 implemented and validated.

## Goal

Turn native ScreenCaptureKit frames into atomic, independently playable,
hardware-H.264 MP4 segments under the approved video-pipeline contract.

## Acceptance Criteria

- A macOS hardware smoke writes a playable H.264 MP4 from native pixel buffers.
- Continuous capture rotates near 10 seconds without buffering replay video in RAM.
- Source, geometry, and material clock discontinuities create clean boundaries.
- Completed segments and pipeline failures are observable at the app seam.

## Context

- [Approved specification](../../specs/2026-08-12-video-pipeline.md)
- [Architecture](../../architecture/index.md)
- [Quality](../../quality.md)

## Tickets

### VP-01 — Prove one native hardware-H.264 segment

Delivered behavior: a native macOS writer accepts `CVPixelBuffer` frames and
atomically produces one fragmented H.264 MP4 without a software fallback.

Acceptance criteria:

- Writer settings require hardware H.264, ~3 Mbps, 30 fps, one-second keyframes,
  no frame reordering, and periodic movie fragments.
- Failed creation, append, or finalization returns a stable error code.
- Completed output appears only after `.partial.mp4` is finalized and renamed.

Validation: focused Rust tests plus a hardware smoke inspected with `ffprobe`.

Blocked by: none.

### VP-02 — Rotate timestamped capture segments

Delivered behavior: the capture worker continuously feeds the writer and emits
completed segment records at time, source, geometry, and discontinuity boundaries.

Acceptance criteria:

- Valid capture PTS becomes strictly monotonic integer microseconds.
- Missing PTS falls back to monotonic arrival time.
- A segment rotates near 10 seconds, on source/geometry change, or on a material
  clock regression/gap; each new file begins near zero.
- Segment records carry path, source, geometry, duration, byte size, monotonic
  session bounds, completion time, and boundary reason.

Validation: deterministic clock/coordinator tests and the hardware smoke.

Blocked by: VP-01.

### VP-03 — Surface pipeline health at the app seam

Delivered behavior: UI and Tauri consumers can distinguish healthy encoding
from capture-only operation and receive completed-segment metadata.

Acceptance criteria:

- Snapshot state exposes idle, encoding, and failed pipeline states plus segment
  count and a stable encoder error code.
- Tauri emits segment-complete and capture-state updates from worker signals.
- The compact rail never labels a failed encoder as recording evidence.

Validation: Rust state tests, Svelte type/build checks, and native smoke.

Blocked by: VP-02.

## Validation

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- macOS ignored hardware smoke followed by `ffprobe`
- `npm run check && npm run build`
- `./scripts/check-sonata.sh`
- `node scripts/check-quality-gates.mjs`

## Decision Log

- 2026-08-12: Internally approved native AVFoundation/VideoToolbox writing over
  a raw-frame FFmpeg pipe; final replay concatenation remains a later concern.
- 2026-08-12: Hardware acceleration is required and has no software fallback.

## Progress Log

- 2026-08-12: VP-01 through VP-03 approved for sequential implementation.
- 2026-08-12: VP-01 implemented; the native smoke produced a 30 fps H.264 MP4
  at 2.90 Mbps with one-second keyframes.
- 2026-08-12: VP-02 implemented; deterministic timeline tests cover duration,
  source, geometry, missing PTS, duplicate PTS, and clock discontinuities.
- 2026-08-12: VP-03 implemented; pipeline health and completed segment events
  reach the Tauri seam and compact rail.
- 2026-08-12: Live capture produced sequential playable 10.02-second H.264
  segments at the display's encoded 1512x982 geometry.
- 2026-08-12: Completion audit added explicit encoder finalization and idle
  state when capture stops. All repository and hardware checks pass.
