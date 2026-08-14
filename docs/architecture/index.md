# Architecture

## Current Shape

- Kind: runnable macOS desktop application with permission onboarding and a
  video-only ScreenCaptureKit source feeding a bounded native sink.
- Stack: Tauri v2 desktop shell with a Svelte 5/TypeScript/Vite interface,
  framework-neutral Motion animations, and a Rust capture and retention core.
  The MVP uses ScreenCaptureKit and AVFoundation/VideoToolbox on macOS; FFmpeg
  packages replay evidence with a bundled FFmpeg sidecar. A native
  `tauri-plugin-global-shortcut` registration triggers replay exports without
  routing OS input through the webview. Platform adapters preserve a path to
  later Windows support.

## Implemented System Map

- `src/main.ts` mounts the Svelte application and global style layers.
- `src/App.svelte` is the stable mount boundary; `src/CaptureShell.svelte`
  projects authoritative Rust permission/capture state into a compact floating
  action rail, lists one-at-a-time display/window sources, and presents typed
  Saving, Saved, Retry, and Finder actions without receiving filesystem paths.
- `src/app.css` owns the transparent-window reset, visual tokens, and floating
  rail presentation.
- `src-tauri/src/lib.rs` exposes typed capture and replay commands, owns the
  typed replay-state event, and registers the fixed global replay shortcut.
- `src-tauri/src/capture/model.rs` owns permission/capture states, diagnostics,
  retry policy, and aspect-fit rules.
- `src-tauri/src/capture/backend.rs` is the fakeable capture boundary;
  `platform.rs` is its macOS ScreenCaptureKit implementation.
- `src-tauri/src/capture/service.rs` owns source switching, interruption
  recovery, and state events. `mailbox.rs` owns the drop-oldest frame handoff.
- `src-tauri/src/encoder/timeline.rs` normalizes capture PTS and decides clean
  segment boundaries. `worker.rs` owns the continuous frame-to-segment loop,
  while `writer.rs` and its Objective-C bridge confine AVFoundation and
  VideoToolbox ownership to the encoder thread.
- `src-tauri/src/retention/` owns directory-derived recovery, completed-segment
  admission, five/ten-minute pruning, storage health, and deletion-safe leases.
- `src-tauri/src/replay/` owns one lease-backed export lifecycle, a bounded
  current saved-replay record, privacy-safe state events, coalescing, retry,
  and opaque-ID Finder reveal.
- `src-tauri/tauri.conf.json` connects Vite's development/build output to the
  native shell, defines the `com.gisketch.encore` application identity, and
  configures a frameless always-on-top window.
- `src-tauri/src/lib.rs` positions that window above the current work area and
  owns the menu-bar/tray Show, Hide, and Quit lifecycle; `desktop.rs` can also
  reveal the rail without focusing it after a global replay trigger.

## Runtime Responsibilities

- Capture selects one display or visible window and emits native video frame
  envelopes with geometry, status, Core Media PTS, and monotonic arrival time.
- Encoder converts frames to H.264 with the platform hardware encoder and a
  one-second keyframe interval.
- Segment retention persists 10-second fragmented-MP4 chunks and deletes chunks
  outside the selected replay window.
- Replay trigger leases the current retained window through the same native
  service method for the rail and global `CommandOrControl+Alt+R` shortcut,
  then dispatches packaging to a worker without blocking either entry point.
- Packager validates one leased window's stream layout, then uses a local FFmpeg
  sidecar to concatenate compatible chunks with stream copy into a transactional
  MP4 plus schema-v1 metadata bundle under `app.path().video_dir()/Encore`.
  Replay owns state transitions and releases the lease only after success.
- Desktop UI owns capture selection, retention settings, permission guidance,
  and privacy-safe replay status. It enables replay only for healthy nonempty
  retention and never receives segment paths. Animation is presentation-only
  and must respect the operating system's reduced-motion preference.
- Desktop presence is intentionally compact: macOS runs as a menu-bar accessory
  with no Dock or title-bar quit dependency; the same lifecycle menu becomes a
  Windows system-tray icon when the Windows capture backend arrives.
- Platform adapters own Windows/macOS capture, hardware encoder, and permission
  differences; the retention and packaging policy remains platform-neutral.

Replay is native and automatic: a trigger begins one asynchronous save. Failed
evidence remains leased for Retry; success retains only an opaque ID and display
name for `open -R`, never a native path in the webview.

## Current Boundaries

| Boundary | Owns | Allowed dependencies | Validation |
|---|---|---|---|
| Desktop UI | Permission guidance, source selection, capture health, presentation | Svelte, Motion, Tauri API | `npm run check && npm run build` |
| Capture service | Switching, recovery, authoritative state and redacted diagnostics | Backend and permission traits, Tauri events | Rust tests and native launch |
| macOS backend | TCC, source mapping, native frame envelopes | macOS 14+, `screencapturekit` 8.0.1 | Fake-boundary tests plus manual matrix |
| Frame handoff | Bounded, nonblocking newest-frame delivery | Crossbeam channel | Overflow unit test |
| Video pipeline | Hardware H.264, timestamp normalization, atomic 10-second MP4 segments | Native frame handoff, AVFoundation, VideoToolbox | Timeline tests, hardware writer smoke, live `ffprobe` inspection |
| Rolling retention | Startup recovery, bounded pruning, storage health, export leases | Standard filesystem and completed segment records | Temporary-directory integration tests and live restart smoke |
| Replay export | One pending lease, one in-flight package, one saved opaque reveal target, typed aggregate state, native global shortcut | Rolling retention, packager, Tauri events, `tauri-plugin-global-shortcut` | Replay service tests, frontend build, native unfocused-app smoke |
| Evidence packager | Compatibility validation, sidecar concat, hidden-workspace publish, safe metadata | Caller-owned lease, FFmpeg/FFprobe runner, package filesystem | Injected runner/filesystem tests and prepared-sidecar version check |

The global-shortcut plugin is Rust-only in this slice. It does not expose its
JavaScript commands or require a capability permission; `default.json` remains
limited to `core:default`.

## Boundary Rule

For each load-bearing boundary, record:

- What it owns.
- Its public interface.
- Allowed dependencies.
- Relevant validation command.

If a boundary must stay true, enforce it mechanically.
