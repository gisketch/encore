# macOS Capture and Permissions

> Status: **IMPLEMENTED / MANUAL VALIDATION PENDING** — all code slices are
> complete; TCC-dependent and second-OS hardware evidence remains.

## Goal

Deliver the approved macOS capture capability from first-launch permission
through dependable display/window frame delivery, without adding encoding,
retention, export, audio, or global shortcuts.

## Acceptance Criteria

- The approved capture spec is satisfied at its native and desktop-UI seams.
- Each ticket leaves the repository runnable and makes one behavior observable.
- The UI never claims capture is active before the native stream proves it.
- Frames remain native and no captured pixels or window titles enter diagnostics.

## Context Links

- `docs/specs/2026-08-12-macos-capture-permissions.md`
- `docs/architecture/index.md`
- `docs/quality.md`

## Ticket Graph

```text
MAC-01 Permission onboarding
  └─> MAC-02 Automatic main-display capture
        └─> MAC-03 Display/window selection and switching
              └─> MAC-04 Interruption recovery and completion matrix
```

## MAC-01 — Permission Onboarding and Authoritative State — IMPLEMENTED

### Delivered Behavior

A first launch explains Screen Recording access, asks only after the tester
chooses **Enable capture**, and shows truthful permission/restart/denial states
from the Rust boundary in the existing shell.

### Acceptance Criteria

- The application targets macOS 14+ and includes the required capture purpose
  description under the stable Encore application identity.
- Rust owns the permission-state contract and exposes typed commands/events to
  Svelte; frontend state is only a projection.
- Required, denied, restart-required, and granted states each show a concrete
  next action and never display **Recording**.
- Repeated denial does not cause a prompt loop; the tester can open the correct
  System Settings location and quit Encore when restart is required.
- Permission behavior is injectable so state transitions are deterministic in
  tests without mutating the developer machine's TCC database.

### Validation

- Rust state-transition tests and invalid-transition rejection.
- Frontend check/build plus an interaction smoke for every fake permission state.
- Manual first-request, denial, grant/restart, and granted-relaunch evidence.
- Existing Rust formatting, Clippy, tests, and Sonata harness.

### Blocked by

- Nothing.

## MAC-02 — Automatic Main-display Capture to a Native Sink — IMPLEMENTED

### Delivered Behavior

With permission already granted, launching Encore automatically captures the
current main display, includes the cursor and Encore's own windows, and
reports **Capturing** only after a healthy native frame reaches a bounded fake
consumer. Capturing Encore itself prevents an otherwise valid display from
remaining blank when the recorder is the only visible application.

### Acceptance Criteria

- A pinned ScreenCaptureKit Rust dependency produces video-only frames at the
  approved 30 fps / maximum 1920×1080 contract without cropping or upscaling.
- The native frame envelope carries geometry, frame status, presentation time,
  and monotonic arrival time; frame pixels never cross Tauri IPC.
- A bounded nonblocking handoff discards the oldest queued frame under pressure
  and exposes a dropped-frame count.
- `started`, `complete`, and `idle` are healthy; blank/suspended output is paused
  and no synthetic frames are emitted.
- The shell shows source, capture health, and dropped-frame diagnostics while
  keeping save-replay unavailable.

### Validation

- Unit tests for frame classification, aspect-fit calculation, bounded overflow,
  and diagnostic redaction using a fake capture backend.
- Native integration evidence that ordered main-display frames reach a fake sink.
- Manual cursor/exclusion smoke and a ten-minute main-display soak.
- Frontend build/check, Rust formatting/Clippy/tests, and Sonata harness.

### Blocked by

- MAC-01.

## MAC-03 — Display/Window Selection and Safe Switching — IMPLEMENTED

### Delivered Behavior

The tester can choose exactly one real display or one visible window. Changing
sources is make-before-break: the current capture remains active until the new
source emits a complete frame, and a failed switch leaves the old source intact.

### Acceptance Criteria

- The selector lists displays and visible application windows without previews.
- Window capture follows the chosen window between displays and includes cursor
  movement; app-wide multiwindow capture remains unavailable.
- Displayable app/window names exist only in current UI memory. Settings and
  diagnostics contain no window title or captured content.
- Window selection is session-only; a relaunch returns to the main display.
- Move and resize update capture without a false stopped state. An update that
  requires stream restart emits an explicit evidence-gap event.
- Switch cancellation/failure keeps the old source and gives a concrete error.

### Validation

- Source mapping, redaction, switching, rollback, and resize tests with fakes.
- Native integration evidence for one display-to-window and window-to-display
  switch with ordered frames at the sink.
- Manual selector, move, resize, and failed-switch smoke.
- Frontend build/check, Rust formatting/Clippy/tests, and Sonata harness.

### Blocked by

- MAC-02.

## MAC-04 — Interruption Recovery and Completion Matrix — IMPLEMENTED; MATRIX PENDING

### Delivered Behavior

Encore stays truthful through window loss, display disconnection, lock, sleep,
and transient ScreenCaptureKit failures. It pauses or recovers predictably and
provides enough privacy-safe state to understand an evidence gap.

### Acceptance Criteria

- Closed/minimized windows and disconnected displays enter
  `source_unavailable`; capture never silently widens to another source.
- Lock/sleep enter a paused state and wake/unlock attempt the same source again.
- A source that returns in the same process may resume automatically.
- Transient stream failures retry after 1, 2, and 4 seconds, then enter failed
  with a next action; permission/source failures do not retry blindly.
- Diagnostics contain only approved identifiers, geometry, state/error codes,
  retry counts, and dropped-frame counts.
- The complete manual matrix passes on macOS 14 and the current macOS release,
  including ten-minute display and window soaks with bounded memory and a
  responsive UI.

### Validation

- State-machine tests for every pause, unavailable, retry, recovery, and failure
  branch using fake lifecycle and capture inputs.
- Manual minimize/restore, close, display disconnect, lock/unlock, sleep/wake,
  revocation, and retry-exhaustion evidence.
- Recorded soak metrics and final frontend/Rust/Sonata validation suite.

### Blocked by

- MAC-03.

## Validation Strategy

- MAC-01 uses the Critical lane because TCC permission and signed identity are
  security-sensitive boundaries.
- MAC-02 through MAC-04 use the Critical lane because native concurrency,
  background lifecycle, and frame loss affect the evidence contract.
- Every ticket needs public-seam behavior evidence plus relevant build/lint/tests;
  manual evidence supplements rather than replaces deterministic fakes.

## Decision Log

- The user's request to spec and ticket the internally grilled design approves
  the macOS capture contract.
- Four tickets are the smallest credible slices: each produces an observable
  state or real capture behavior and fits a fresh implementation context.
- No standalone abstraction/prefactoring ticket is needed; the fakeable boundary
  is introduced by MAC-01 and exercised immediately.
- Local repository documents are the tracker; no GitHub issues are created.

## Progress Log

- 2026-08-12: Spec approved and initial four-ticket dependency map drafted.
- 2026-08-12: User approved implementation of all tickets; MAC-01 started.
- 2026-08-12: MAC-01 through MAC-04 implemented with ScreenCaptureKit 8.0.1,
  typed Tauri commands/events, a fakeable backend, make-before-break switching,
  bounded drop-oldest delivery, and 1/2/4-second recovery.
- 2026-08-12: Automated Rust/frontend/SCC checks and native launch passed on
  macOS 26.5. Screen Recording grant/restart, capture soaks, interruption
  matrix, and a macOS 14 run remain manual because they require TCC and hardware
  state changes.
- 2026-08-12: Sonata review findings resolved: old-stream events are
  source-scoped, switching and recovery are serialized, rollback checks real
  source availability, resize updates in place with controlled-restart fallback,
  and fake-backend tests cover successful start, rollback, and resize.
- 2026-08-12: Native startup timeout fixed after live evidence showed concurrent
  ScreenCaptureKit enumeration/startup and a crate attachment-reader failure.
  Native operations are serialized, and a missing status with a real pixel
  buffer is treated as complete; explicit blank output still starts paused.
- 2026-08-12: Full-display capture now includes Encore by explicit product
  decision. This removes the persistent blank/paused state caused by excluding
  the only visible application and keeps the display filter minimal.
- 2026-08-12: The large dashboard was replaced by a 760×104 frameless action
  rail anchored above the work-area bottom edge. macOS now uses a menu-bar
  accessory for Show, Hide, and Quit; the same tray lifecycle is retained for
  the future Windows build.
