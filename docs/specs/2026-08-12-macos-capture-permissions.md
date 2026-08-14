# macOS Capture and Permissions

> Status: **APPROVED** — canonical contract for macOS capture implementation.

## Problem and Desired Outcome

Encore cannot preserve replay evidence until it can reliably obtain frames from
a selected macOS display or window. The capture layer must begin automatically
after one-time consent, keep frame data native, and report permission, source,
and stream failures without ever claiming stale or nonexistent capture.

## Users and Operating Environment

- Primary user: a QA tester who should not need to remember to start recording.
- First platform: macOS 14 Sonoma or later.
- Expected use: long-running local capture while another application or game is
  focused, including normal sleep, wake, lock, unlock, and display changes.

## In Scope

- One-time macOS Screen Recording TCC permission and restart guidance.
- An Encore-owned selector for exactly one display or one visible window.
- Video-only ScreenCaptureKit stream lifecycle and native frame delivery.
- Automatic main-display capture after permission on future launches.
- Cursor capture, source switching, source loss, sleep/wake, and stream recovery.
- Typed capture state and privacy-safe diagnostics for the desktop shell.
- A bounded native frame handoff for the later encoder boundary.

## Out of Scope

- Encoding, segment files, retention, export, hotkeys, audio, and Windows.
- App-level capture that combines every window belonging to an application.
- Frame previews, screenshots, thumbnails, or pixels sent to the webview/logs.
- The per-session `SCContentSharingPicker` flow for the MVP.

## Observable Acceptance Behavior

### Permission

- Before requesting access, Encore explains why capture is needed and remains in
  `permission_required`; it does not claim to be recording.
- The tester explicitly chooses **Enable capture** before macOS is asked for
  Screen Recording permission.
- A grant that requires restart produces `restart_required` with actions to open
  the correct System Settings pane and quit Encore. Encore does not fake an
  in-process success.
- Denial or later revocation produces `permission_denied`, does not repeatedly
  prompt, and offers the System Settings action.
- On a later launch with permission, capture starts without another picker or
  start-recording action.

### Source Selection

- A permitted fresh launch targets the current main display by default.
- The tester may switch to exactly one listed display or one currently visible
  application window.
- Display labels and window app/title text may appear in the selector, but
  window titles are session-only and never written to settings or diagnostics.
- Full-display capture includes Encore's own windows. Capturing the recorder UI
  is acceptable and avoids blank output when Encore is the only visible window.
- Window capture uses a desktop-independent window filter and follows the window
  as it moves between displays.
- Cursor movement is included because it is useful QA evidence.
- A source switch keeps the old stream active until the replacement emits a
  complete frame. If replacement fails, the old source remains active and the
  failure is shown.

### Capture Health

- `complete`, `started`, and `idle` frames are healthy. An unchanged display is
  not an error merely because ScreenCaptureKit reports an idle frame.
- If the pinned binding cannot decode the status attachment but provides a real
  pixel buffer, the frame is treated as complete instead of blank.
- Blank or suspended output produces a visible paused state and emits no
  synthetic frames.
- Lock and sleep pause capture; unlock and wake attempt to resume the same source
  automatically.
- A closed/minimized selected window or disconnected selected display produces
  `source_unavailable`. Encore never silently switches to broader content.
- If that same source becomes available again during the process lifetime,
  Encore may resume it automatically; otherwise the tester selects a new source.
- A resize updates the stream configuration without presenting a false stopped
  state. If the update fails, Encore performs a controlled stream restart and
  exposes the resulting evidence gap.
- Transient internal stream failures enter `recovering` and retry three times
  after 1, 2, and 4 seconds. Permission and missing-source failures are not
  retried blindly. Exhaustion enters `failed` with a concrete next action.

### Desktop Presence

- The primary interface is one frameless, always-on-top action rail positioned
  above the current display's usable bottom edge.
- The rail exposes only current capability: capture health, source selection,
  5/10-minute intent, permission/retry actions, and a visibly unavailable save
  action until replay retention exists.
- On macOS, Encore runs as an accessory application with Show, Hide, and Quit
  in its menu-bar item. Closing the rail hides it without stopping capture.
- The same lifecycle menu is implemented through Tauri's tray boundary for the
  future Windows build.

### Native Handoff

- The capture boundary delivers complete video frames with source geometry,
  ScreenCaptureKit frame status, presentation timestamp, and a monotonic arrival
  timestamp to a native consumer; no frame pixels cross Tauri IPC.
- Capture targets 30 frames per second and at most a 1920×1080 bounding box,
  preserving source aspect ratio without cropping or upscaling.
- The callback never blocks on the future encoder. A bounded handoff discards
  the oldest queued frame on overflow, increments an observable dropped-frame
  counter, and keeps the most recent evidence moving.
- Source-resize and stream-restart discontinuities are explicit native events so
  the video-pipeline spec can decide how to preserve a stable encoded format.

## State Contract

Permission state:

- `unknown`
- `permission_required`
- `restart_required`
- `granted`
- `permission_denied`

Capture state:

- `stopped`
- `starting`
- `capturing`
- `paused(reason)`
- `recovering(attempt, reason)`
- `source_unavailable(reason)`
- `failed(reason, next_action)`

The Svelte shell may request permission, list sources, start/switch/stop capture,
and read state. Native capture-state events are authoritative; frontend state is
never evidence that capture is active.

## Data and Privacy Contract

- The capture subsystem performs no network access.
- Pixels and source thumbnails never enter logs, settings, or the Svelte layer.
- Settings may persist the source kind and display identifier, but never a
  window title. Window selection lasts only for the current process session.
- Diagnostics may record timestamps, source kind, opaque source identifier,
  geometry, state transitions, error codes, retry attempts, and dropped-frame
  counts. They must not record window titles or captured content.
- A subsequent launch always falls back to the current main display rather than
  attempting to rediscover and silently capture a prior window.

## Implementation Constraints and Settled Decisions

- Use ScreenCaptureKit through exactly pinned `screencapturekit` 8.0.1 with its
  macOS 14 feature and Rust/native boundary.
- Use standard Screen Recording TCC access with an
  `NSScreenCaptureUsageDescription`; TCC permission, not an entitlement, grants
  capture access.
- Use Encore's selector because automatic later launches are a core behavior.
  Apple's system picker is intentionally deferred because its consent applies
  to a selected capture session and would reintroduce tester discipline.
- Keep capture callbacks off the UI thread and make their downstream handoff
  bounded and nonblocking.
- Include Encore in full-display capture; avoiding a blank or paused stream is
  more important than preventing the recorder UI from appearing in evidence.
- Prefer explicit stopped/paused states over frozen last-frame presentation.

## Expected Validation Evidence

- Unit tests for state transitions, retry exhaustion, overflow counting, source
  redaction, and invalid-transition rejection using a fake capture backend.
- Integration evidence that complete frames with ordered timestamps reach a
  fake native consumer from one display and one window.
- Manual TCC matrix: first request, denial, grant/restart, granted relaunch, and
  revocation while Encore is installed as the same signed application identity.
- Manual source matrix: display, window, cursor, move, resize, minimize/restore,
  close, display disconnect, lock/unlock, and sleep/wake.
- A ten-minute display soak and ten-minute window soak with bounded memory,
  stable UI responsiveness, and recorded dropped-frame counts.
- Run the matrix on the minimum supported macOS 14 release and the current macOS
  release before calling the capability complete.

## Remaining Risks

- Apple recommends its system picker; the automatic custom-picker decision is
  intentional but should receive extra privacy review before distribution.
- Window minimization and protected-content behavior may differ by macOS release
  or application and must be confirmed with real integration evidence.
- The encoder must define how varying source geometry maps into stable segment
  dimensions; this spec reports geometry changes but does not own that policy.
- Codesigning identity must remain stable during TCC tests or permission results
  will be misleading.
