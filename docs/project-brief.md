# Project Brief

Encore is a local desktop replay buffer for capturing the evidence immediately
before a rare QA failure.

## Product Vision

Make rare bugs reproducible without asking testers to predict them. Encore
continuously retains only a short rolling window; one global hotkey preserves
that window as a durable, locally saved evidence bundle for a developer.

## Users

- Primary user: QA testers exercising desktop software and games.
- Secondary users: developers investigating the captured failure.
- Operating environment: macOS first, followed by Windows; Encore must operate
  while it is not the focused application.

## Current Milestone

- Outcome: prove a local, video-only rolling capture MVP on macOS.
- Acceptance behavior:
  - Launching Encore starts a rolling capture with a default full-screen target
    and a 10-minute retention window.
  - The tester can choose full-screen or one application window and can choose
    a 5- or 10-minute retention window.
  - Capture writes 10-second segments to disk and prunes expired segments so
    ordinary recording does not grow without bound.
  - A global hotkey works while Encore is unfocused and packages the retained
    segments into a playable local MP4 without re-encoding them.
  - The saved evidence includes build version, OS details, timestamp, capture
    settings, and available local logs in adjacent machine-readable metadata.
  - Segments live on disk rather than only in memory, so useful pre-failure
    evidence remains available when the software under test crashes.
  - The application makes permission failures and export failures observable
    and actionable to the tester.

## Problem

Rare failures disappear before QA can document them. Eight-hour recordings are
too large, screenshots omit the input sequence that caused the failure, and
reproduction steps written afterward are unreliable. Developers then chase
incomplete evidence and edge cases are closed as cannot reproduce.

## Non-Goals

- Cloud storage, accounts, telemetry, or any required network service.
- Direct Jira, Linear, S3, or other ticket-system upload.
- Saving an unbounded recording session.
- Audio capture in the MVP.
- Software encoding as the normal production path.

## Later / Not Now

- System audio capture and explicit wall-clock A/V synchronization.
- Windows support using Windows Graphics Capture and NVENC/QSV/AMF.
- Optional ticket-system or object-storage upload integrations.
- Additional capture controls beyond choosing a full display or one window.

## Constraints

- Source: public GitHub repository at `gisketch/encore`.
- Issue tracking: local repository documents only.
- Stack: Tauri v2 shell with a Rust core; Windows Graphics Capture through
  `windows-capture`; ScreenCaptureKit through `screencapturekit-rs`; hardware
  H.264 via NVENC/QSV/AMF or VideoToolbox; bundled FFmpeg sidecar for muxing;
  `tauri-plugin-global-shortcut` for the trigger.
- Package manager: Cargo for Rust; JavaScript package manager pending selection.
- Runtime: a native local macOS desktop process for the MVP, with platform
  boundaries retained for later Windows support.
- Data: rolling 10-second fragmented-MP4 segments plus local exports and
  metadata. Expired rolling segments are deleted; saved exports remain until
  the user deletes them.
- Security: screen contents and logs remain local; request only the OS capture
  permissions needed to operate. macOS must guide the user through Screen
  Recording permission and the required first-grant restart.
- Performance: target 1080p30 at approximately 3 Mbps, a one-second keyframe
  interval, hardware H.264, bounded rolling disk use of roughly 250 MB for 10
  minutes, and negligible steady-state CPU overhead on supported hardware.
- Delivery: video-only MVP first; preserve boundaries needed for later audio.

## Open Questions

- None currently.
