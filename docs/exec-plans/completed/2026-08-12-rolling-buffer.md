# Rolling Buffer and Crash Recovery Plan

## Goal

Implement the approved disk-backed rolling store so Encore recovers completed
evidence after a crash, bounds normal disk use to five or ten minutes, and
reports retention truthfully through the floating control.

## Acceptance Criteria

- The behavior in the [rolling-buffer specification](../../specs/2026-08-12-rolling-buffer.md)
  is observable at the filesystem, Rust service, and desktop UI seams.
- Every ticket's focused validation and the critical persistence lane pass.
- The native ScreenCaptureKit/VideoToolbox backend remains unchanged.

## Context

- [Rolling-buffer specification](../../specs/2026-08-12-rolling-buffer.md)
- [Architecture](../../architecture/index.md)
- [Quality menu](../../quality.md)
- [libobs migration assessment](../../research/2026-08-12-libobs-migration-assessment.md)

## Tickets

### RB-01 — Recover and bound the rolling directory

**Delivered behavior:** A platform-neutral Rust store recovers atomically
published segments, cleans interrupted files, prunes to the selected duration,
and protects leased snapshots from deletion.

**Acceptance criteria:**

- Startup admits ordered, regular, non-empty `segment-*.mp4` files and removes
  `.partial.mp4` plus zero-byte rolling files.
- Admission rejects paths outside the rolling directory and prunes to the
  selected window plus at most one boundary segment.
- Five/ten-minute changes prune immediately where required.
- A stable ordered lease prevents deletion until released.
- Failures produce stable privacy-safe error codes.

**Validation:** Focused filesystem tests in unique temporary directories,
followed by Rust formatting, Clippy, and the complete Rust test suite.

**Blocked by:** None.

### RB-02 — Make retention authoritative in the app

**Delivered behavior:** Completed encoder segments flow through the rolling
store, capture snapshots expose real retention health/statistics, and the
floating duration selector controls the Rust policy.

**Acceptance criteria:**

- Service startup exposes recovered segment count and bytes.
- Every segment-complete signal is admitted before the UI receives its updated
  authoritative state.
- The Tauri command accepts only five or ten minutes.
- The floating control mirrors core state and visibly reports retention failure.
- Architecture and quality documentation reflect the implemented boundary and
  reproducible checks.

**Validation:** Capture-service tests, `npm run check`, `npm run build`, complete
Rust checks, Sonata harness, and changed-code quality gates.

**Blocked by:** RB-01.

## Validation Lane

Critical: this slice owns persistence and deletion. Run focused filesystem
integration evidence first, then all Rust checks plus frontend type/build checks
and repository gates.

## Decision Log

- 2026-08-12: Keep the native media backend; libobs is deferred to an explicit
  Windows/audio/composition decision gate.
- 2026-08-12: Use the rolling directory as authority rather than a mutable index.
- 2026-08-12: Keep one oldest boundary segment and use leases for prune safety.
- 2026-08-12: The user's explicit full-workflow instruction approves both local
  tickets in blocker order; no external tracker publication is configured.

## Progress Log

- 2026-08-12: Internal grill completed; canonical spec approved.
- 2026-08-12: RB-01 and RB-02 drafted and approved for local implementation.
- 2026-08-12: RB-01 implemented with deterministic recovery, five/ten-minute
  pruning, duration-change, path-rejection, and lease-safety coverage.
- 2026-08-12: RB-02 implemented through the capture snapshot, Tauri command,
  segment-complete monitor, and floating duration control.
- 2026-08-12: Critical validation lane passes: 30 Rust tests with one ignored
  hardware smoke, Clippy, frontend checks/build, Sonata harness, SCC, and diff
  whitespace checks.
- 2026-08-12: First Sonata review found a P1 cutoff error that allowed two extra
  chunks. Pruning now anchors the window at the newest segment end; focused
  five/ten-minute expectations prove the exact bound.
- 2026-08-12: Second Sonata review found that a permanent retention failure
  could allow unbounded new files. Failed startup queues an encoder halt, and a
  failed live admission halts encoding while leaving screen capture responsive.
- 2026-08-12: Third review found a false-recovery edge after the halt and a
  duration-setting failure gap. Retention failure is now process-sticky, and
  both admission and setting failures use the same encoder-halt path.
- 2026-08-12: UI seam review replaced the misleading post-halt "Starting
  encoder" state with an authoritative "Recording failed" presentation.
- 2026-08-12: Destructive-path review narrowed crash cleanup from every
  `.partial.mp4` to Encore-owned `segment-*.partial.mp4`; a regression test
  proves unrelated completed and partial files remain untouched.
- 2026-08-12: Final Sonata review found no remaining Standards, Spec, or
  Behavior defects; RB-01 and RB-02 are complete.
