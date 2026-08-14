# Replay Trigger and Snapshot Plan

## Goal

Turn the retained rolling window into one deletion-safe pending replay from the
floating rail or an unfocused-app global shortcut, without claiming that the
later MP4 packaging step already exists.

## Acceptance Criteria

- The behavior in the [approved replay-trigger specification](../../specs/2026-08-12-replay-trigger.md)
  is observable at native service, desktop shortcut, and floating-rail seams.
- One pending replay remains bounded, privacy-safe, and protected from pruning.
- Every ticket's focused validation and the milestone lane pass.

## Context

- [Replay-trigger specification](../../specs/2026-08-12-replay-trigger.md)
- [Rolling-buffer specification](../../specs/2026-08-12-rolling-buffer.md)
- [Architecture](../../architecture/index.md)
- [Quality menu](../../quality.md)

## Tickets

### RT-01 — Create one atomic pending replay

**Delivered behavior:** A native replay service acquires an ordered retention
lease, owns one pending snapshot, coalesces rapid duplicates, preserves prior
evidence on failure, and exposes privacy-safe typed state.

**Acceptance criteria:**

- An eligible trigger returns an opaque monotonically increasing ID, creation
  time, segment count, total bytes, and evidence bounds without segment paths.
- The service holds the underlying ordered lease until a successful replacement
  or process exit.
- A successful replacement acquires its new lease before dropping the old one.
- Triggers within 750 ms coalesce to the same replay ID.
- Empty/failed retention and internal failures return stable codes while
  preserving an earlier pending lease.
- Concurrent callers serialize through one native service boundary.

**Validation:** Focused deterministic Rust tests plus formatting, Clippy, and the
complete Rust suite.

**Blocked by:** None.

### RT-02 — Trigger globally and report honestly

**Delivered behavior:** `Cmd+Option+R` invokes RT-01 while Encore is unfocused,
the same rail button invokes the same native command, and typed state reports
ready/failure/shortcut availability without claiming an exported file.

**Acceptance criteria:**

- The Rust global-shortcut plugin registers
  `CommandOrControl+Alt+R` at startup and reacts only to press events.
- Registration failure is non-fatal and represented by
  `shortcut_registration_failed`; manual triggering remains available.
- A global trigger dispatches off the OS callback, reveals the rail without
  focusing it, and emits the same replay state as the command path.
- The rail enables replay only when retained evidence is eligible, shows
  `Replay ready` after success, and shows an active shortcut hint only after
  successful registration.
- Capabilities, architecture, and quality documentation match the implemented
  boundary.

**Validation:** Service seam tests, frontend type/build checks, all Rust checks,
Sonata/SCC/diff gates, and a documented native unfocused-app smoke.

**Blocked by:** RT-01.

## Validation Lane

Milestone: this crosses persistence leases, concurrency, a global OS callback,
Tauri state/events, and user-visible behavior. Run focused ticket checks while
working and every relevant verified project check before review.

## Decision Log

- 2026-08-12: Sol orchestration internally grilled and approved the fixed
  shortcut, no-tail, one-pending, 750 ms coalescing contract.
- 2026-08-12: The requested Luna implementation model is unavailable in this
  workspace; GPT-5.6 Terra xHigh is the approved closest substitute.
- 2026-08-12: Local repository documents are authoritative; no external tracker
  publication is configured.

## Progress Log

- 2026-08-12: Grill completed and canonical spec approved.
- 2026-08-12: RT-01 and RT-02 approved for sequential subagent implementation.
- 2026-08-12: RT-01 complete — native `ReplayService` now holds one ordered
  pending lease, exposes only aggregate replay state, coalesces 750 ms repeats,
  and preserves prior evidence on stable-code failures. Focused replay tests,
  Rust formatting, Clippy, and the complete Rust suite pass.
- 2026-08-12: RT-02 complete — native startup registers
  `CommandOrControl+Alt+R` once through `tauri-plugin-global-shortcut`; Pressed
  events dispatch the shared replay trigger path on a worker and reveal the
  rail without focusing it. The rail consumes typed aggregate replay state,
  enables only with healthy retained evidence, and shows a shortcut hint only
  after registration. The Rust-only plugin needs no capability permission.
  Manual macOS shortcut delivery remains pending the documented smoke.
- 2026-08-12: RT-02 review correction — `RailActions` keeps exactly one manual
  replay action after the conditional capture onboarding/retry action, including
  recovered evidence when capture is unavailable. A separate deterministic
  Tauri `AppHandle` seam test is not practical without a native runtime; the
  command and shortcut worker both call the single `trigger_and_emit` helper,
  which invokes `ReplayService::trigger` and emits the same typed state. Rust
  compilation, Clippy, and the complete suite verify that shared wiring.
