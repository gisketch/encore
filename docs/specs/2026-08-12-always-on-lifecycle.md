# Always-on Lifecycle and Control

> Status: **APPROVED** — internally grilled 2026-08-13.

## Problem and Outcome

The capture pipeline only solves the QA problem if it operates without ongoing
attention and tells the truth when it cannot. The desired outcome is a coherent
desktop lifecycle: Encore survives a full tester workday — launch, hide, sleep,
wake, source loss, pause, relaunch — while its settings persist, its state is
observable, and every failure has a visible recovery path.

## In Scope

- Launch, close-window, quit, sleep/wake, and source-loss behavior.
- Persisting the capture target choice and retention duration across relaunch.
- Explicit pause/resume that never silently discards retained evidence.
- Truthful lifecycle presentation in the rail and tray using the existing typed
  state model (permission, capture, retention, replay).
- Structured local diagnostic logging sufficient to explain a failed day.

## Out of Scope

- Cloud services, telemetry, automatic updates, Windows, and system audio.
- Launch-at-login and login-item consent (future milestone).
- Shortcut rebinding and export-destination selection; both stay fixed this
  milestone (`CommandOrControl+Alt+R`, `~/Movies/Encore`).
- System notifications; the rail and tray are the only feedback surfaces.
- Capturing locked-screen content or making claims about it.

## Acceptance Criteria

- Launching Encore with granted permission and valid settings starts rolling
  capture into the persisted target and retention window with no tester input.
- Closing the window hides it; capture, retention, and the global shortcut
  continue unchanged. The tray restores the window. This is the only
  close-window behavior; quitting happens only through the tray or an explicit
  quit command.
- Encore is a menu-bar accessory: no Dock icon, no app switcher entry, and the
  floating rail never steals focus when revealed by lifecycle events.
- The persisted settings are the capture target choice and retention minutes.
  They survive relaunch. A missing or corrupt settings file yields defaults
  (full screen, 10 minutes) without blocking startup.
- A persisted window target that cannot be resolved at launch falls back to the
  default display and shows a visible source notice; Encore never silently
  records a different target than the UI claims.
- Explicit pause stops new segment production and freezes wall-clock pruning so
  the evidence retained at pause time remains replay-triggerable. Resume
  restarts capture into the same target and resumes normal pruning.
- System sleep transitions capture to a recovering state. On wake, Encore
  auto-restarts capture into the same target with bounded retries; exhausted
  retries land in a failed state with a one-click retry action.
- Losing the capture source (window closed, display disconnected) transitions
  to `source_unavailable` with visible actions: retry, pick another source, or
  fall back to full screen. No silent target switching.
- Quit is immediate: the pending replay lease is released, in-progress segment
  files are finalized or left for crash recovery, and the next launch recovers
  rolling segments per the rolling-buffer spec.
- Every lifecycle transition and failure above is appended to a structured
  local log with a timestamp, typed state, and stable error code, capped in
  size, and stored under the app's local log directory.
- The UI never advertises capture, replay, or save availability that the
  native state machine has not confirmed.

## Implementation Constraints and Settled Decisions

- Reuse the existing typed enums (`PermissionState`, `CaptureState` including
  `Paused`, `Recovering`, `SourceUnavailable`, `RetentionState`, replay
  states); lifecycle work extends transitions, not the vocabulary, unless a
  transition is unrepresentable.
- Settings persist as one versioned JSON document written atomically
  (write-temp-then-rename) in the app config directory. Unknown fields are
  preserved-on-read or ignored, never fatal. Window targets persist as
  best-effort identity (app bundle plus title), resolved at launch.
- Pause is a user intent, distinct from failure states. While paused, the
  retention pruner idles; disk stays bounded because no new segments arrive.
- Wake recovery reuses the same resume path as user-initiated resume; sleep,
  wake, and stream-error recovery share one bounded retry policy instead of
  parallel mechanisms.
- Lock/unlock gets no special handling: capture continues with whatever frames
  macOS supplies, and stream errors route through the shared recovery path.
- The diagnostic log is JSON Lines, local-only, size-capped with rotation, and
  may contain local paths; the webview continues to receive only privacy-safe
  typed state and stable codes.
- No new windows, dialogs, or notification frameworks; tray menu items and the
  existing rail present all lifecycle state and actions.

## Expected Validation

- Deterministic Rust tests: settings round-trip, corrupt-file fallback, atomic
  write, unresolvable-window fallback, pause freezing pruning, resume, bounded
  wake-retry exhaustion, and source-loss transition actions.
- Frontend checks/build cover paused, recovering, source-unavailable, and
  fallback-notice presentation without local paths.
- Manual macOS smokes: relaunch persistence, close-then-tray-restore, real
  sleep/wake recovery, window-target loss, and log inspection after a failure.
- Rust formatting, Clippy, complete tests, Sonata harness, SCC, and diff checks
  pass before review.

## Risks and Open Questions

- macOS sleep/wake signals reach ScreenCaptureKit indirectly; the retry policy
  must key off stream errors and clock discontinuities, not only power events.
- Freezing pruning during long pauses keeps old evidence by design; testers may
  be surprised that a resumed session prunes it quickly afterward. Rail copy
  must state the retained-window age honestly.
- Best-effort window identity can resolve to the wrong window after the target
  app relaunches; the visible source notice is the guard, not resolution magic.
- Launch-at-login remains the largest gap between "always-on" and reality; it
  is deliberately deferred until the manual daily loop is proven.

## Internal Grill Record

1. **Is launch-at-login part of this slice?** No. It adds login-item consent
   and packaging surface before the manual daily loop is proven. Deferred.
2. **What does closing the window do?** Hide only — already implemented and
   consistent with always-on intent. Quit stays an explicit tray action.
3. **Dock or menu bar?** Menu-bar accessory only — already implemented; a Dock
   presence invites Cmd-Q reflexes that would kill capture mid-day.
4. **What does pause mean?** User intent to stop producing evidence while
   keeping what exists. Pruning freezes so pause never destroys evidence; disk
   stays bounded because nothing new is written.
5. **What happens on sleep and wake?** Recovering state, then auto-resume with
   bounded retries through the same path as manual resume. One recovery
   mechanism, not three.
6. **What about lock/unlock?** Nothing special. macOS decides what frames
   exist; Encore makes no claims about locked-screen content.
7. **What happens when the captured window disappears?** Visible
   `source_unavailable` with retry, re-pick, or full-screen fallback. Silent
   target switching would break the honesty constraint.
8. **Which settings persist?** Capture target choice and retention minutes —
   the two the tester actually changes. Export destination and shortcut stay
   fixed constants this milestone, so persisting them would be dead schema.
9. **How do settings survive corruption?** Atomic writes plus
   default-on-corrupt. Defaults are safe (full screen, 10 minutes), so startup
   never blocks on a settings problem.
10. **Are there system notifications?** No. The always-on-top rail plus tray
    already deliver feedback without adding a permission prompt surface.
11. **What does quit do to evidence?** Releases the pending lease and relies on
    existing crash recovery for segments; no quit confirmation dialog, because
    quit is only reachable through a deliberate tray action.
12. **What must the log answer?** "Why did today's capture stop?" — timestamped
    typed transitions with stable codes for permission, capture, retention,
    and export, locally and bounded in size.
