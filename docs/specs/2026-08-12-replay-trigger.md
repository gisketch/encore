# Replay Trigger and Snapshot

> Status: **APPROVED** — internally grilled 2026-08-12.

## Problem and Outcome

The tester must preserve evidence at the moment a rare failure appears without
focusing Encore or racing the retention pruner. The desired outcome is one
global action that freezes a coherent replay-window snapshot while capture
continues and reports honestly that the snapshot is ready for the later
packaging step.

## In Scope

- One fixed global shortcut registered by the native desktop application.
- The same trigger action exposed through the floating Encore rail.
- Atomic selection and leasing of the completed rolling segments belonging to
  one replay request.
- One bounded pending replay, deterministic duplicate behavior, typed state,
  and visible success or failure feedback.
- A backend handoff that the local-evidence packager can consume later without
  exposing local segment paths to the webview.

## Out of Scope

- MP4 concatenation, metadata bundles, saved-file naming, and opening exports.
- Shortcut rebinding, multiple queued replays, and durable pending snapshots.
- Post-trigger recording tails, system notifications, audio, and online work.

## Acceptance Criteria

- On macOS, `Cmd+Option+R` triggers while another application is focused. The
  platform-neutral accelerator is `CommandOrControl+Alt+R`, reserving
  `Ctrl+Alt+R` for a future Windows build.
- A manual rail action invokes the exact same Rust trigger path as the shortcut.
- A successful trigger atomically acquires an ordered lease over every currently
  retained segment and exposes a privacy-safe snapshot containing an opaque ID,
  creation time, segment count, total bytes, and evidence time bounds.
- Pruning cannot remove leased files. A later successful trigger acquires its
  new lease before replacing and releasing the previous pending replay.
- There is no post-trigger tail: the snapshot contains only segments completed
  when the trigger is accepted, and capture/encoding continue normally.
- Repeated triggers accepted within 750 ms are coalesced into the existing
  result and do not replace its lease or increment its identifier.
- Triggering with no retained segments, failed retention, or another internal
  failure returns a stable local error code and preserves any previous pending
  replay lease.
- A shortcut registration conflict or unsupported platform does not prevent app
  startup or manual triggering. The UI does not advertise the shortcut as
  active unless native registration succeeds.
- A successful global trigger shows the floating rail without focusing it and
  displays `Replay ready`; failures display an actionable local error state.
- No UI text says an MP4 or saved clip exists in this slice. A pending replay is
  memory-owned and is lost on app exit, while its underlying rolling segments
  remain governed by normal crash recovery.

## Implementation Constraints and Settled Decisions

- Use `tauri-plugin-global-shortcut` through its Rust API. Registration and the
  shortcut handler remain native; the webview receives typed state/events only.
- React only to a shortcut press, not its release. Registration occurs once at
  startup and remains fixed for the MVP.
- Hold at most one pending `SegmentLease`. This bounds temporary disk growth to
  the rolling window plus one pending snapshot and avoids an unbounded queue.
- Lease acquisition and pending-snapshot replacement happen behind one service
  synchronization boundary. Failed attempts never discard the previous lease.
- The shortcut handler must not block the OS callback on filesystem or UI work;
  it dispatches the trigger through a short-lived worker.
- UI-visible state contains counts, byte totals, timestamps, shortcut status,
  and stable codes only. Segment paths and source labels stay native.
- The button is available when retention is healthy and at least one segment is
  retained, even if shortcut registration failed.
- `replay_unavailable`, `replay_retention_failed`,
  `replay_trigger_failed`, and `shortcut_registration_failed` are stable local
  codes for this boundary.

## Expected Validation

- Deterministic Rust tests cover empty stores, ordered leases, privacy-safe
  snapshots, 750 ms coalescing, atomic replacement, failed-attempt preservation,
  and concurrent trigger serialization.
- Service tests prove manual and shortcut entry points share one trigger path.
- Frontend checks/build cover truthful ready, unavailable, and shortcut-conflict
  presentation without local paths.
- A manual native smoke hides or unfocuses Encore, presses `Cmd+Option+R`, and
  observes the rail reappear without focus plus one `Replay ready` transition.
- Rust formatting, Clippy, complete tests, Sonata harness, SCC, and diff checks
  pass before review.

## Risks and Open Questions

- Global accelerators can conflict with other applications. Rebinding is
  deliberately deferred; the manual button remains the fallback.
- The app cannot provide a durable saved artifact until local packaging exists.
  UI language must continue to distinguish `Replay ready` from `Saved`.
- The pending lease intentionally increases disk use until it is replaced or
  the process exits. The one-pending limit keeps that increase bounded.
- Native shortcut behavior still needs a manual macOS smoke because unit tests
  cannot prove delivery while another application owns focus.

## Internal Grill Record

1. **What does this slice deliver without packaging?** A deletion-safe pending
   replay, not an MP4. The UI says `Replay ready`, never `Saved`.
2. **What is the default shortcut?** `CommandOrControl+Alt+R`: memorable for
   Replay and less collision-prone than browser refresh or macOS screenshot
   shortcuts.
3. **Is rebinding included?** No. Zero-setup behavior and one reliable native
   path matter more for the MVP; conflicts retain a manual-button fallback.
4. **Is there a post-trigger tail?** No. The user acts after seeing the bug, and
   a tail would add delay and another completion state before it adds evidence.
5. **How many pending replays exist?** One. Multiple leases would grow disk use
   without a packager capable of consuming them.
6. **What does a later trigger do?** Acquire the new lease first, then replace
   the old pending replay so pruning never sees a gap between them.
7. **What do rapid repeats do?** Coalesce for 750 ms and return the same ID.
   This absorbs key repeat and double-clicks without hiding later distinct bugs.
8. **What happens on trigger failure?** Preserve any existing pending evidence
   and expose a stable error; never trade known evidence for a failed attempt.
9. **Can a tester trigger after capture stops?** Yes, if healthy recovered
   segments exist. Eligibility follows retained evidence, not capture focus.
10. **What happens if registration conflicts?** App startup continues, shortcut
    state is unavailable, no active shortcut hint is shown, and the button works.
11. **Should a hotkey steal focus?** No. It may reveal the always-on-top rail so
    feedback is visible, but the tested application keeps keyboard focus.
12. **Are paths sent to Svelte?** No. The native pending lease owns paths; the
    webview receives only privacy-safe aggregate state.
