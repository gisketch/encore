# Architecture

## Current Shape

- Kind: greenfield Sonata harness; no runnable application shell yet.
- Stack: Tauri v2 desktop shell with a Svelte 5/TypeScript/Vite interface,
  framework-neutral Motion animations, and a Rust capture and retention core.
  The MVP uses ScreenCaptureKit and VideoToolbox on macOS; FFmpeg is a bundled
  local muxing sidecar. Platform adapters preserve a path to later Windows
  support.

## System Map

Only planned runtime responsibilities are known so far:

- Capture selects a full display or application window and emits timestamped
  video frames through the platform capture API.
- Encoder converts frames to H.264 with the platform hardware encoder and a
  one-second keyframe interval.
- Segment retention persists 10-second fragmented-MP4 chunks and deletes chunks
  outside the selected replay window.
- Trigger receives a global hotkey while Encore is unfocused.
- Packager concatenates retained chunks without re-encoding and writes a local
  MP4 with adjacent metadata and available logs.
- Desktop UI owns capture selection, retention settings, permission guidance,
  status, and access to saved clips. Animation is presentation-only and must
  respect the operating system's reduced-motion preference.
- Platform adapters own Windows/macOS capture, hardware encoder, and permission
  differences; the retention and packaging policy remains platform-neutral.

These are responsibility boundaries, not implemented modules. Entry points,
interfaces, and dependency directions will be recorded when the runnable shell
exists.

## Boundary Rule

For each load-bearing boundary, record:

- What it owns.
- Its public interface.
- Allowed dependencies.
- Relevant validation command.

If a boundary must stay true, enforce it mechanically.
