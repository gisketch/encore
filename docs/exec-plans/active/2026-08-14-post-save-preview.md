# Post-Save Preview Plan

Status: **READY** — spec approved (self-grilled 2026-08-14), tickets below.

## Goal

Make a saved replay announce itself: a confirmation sound and a corner
picture-in-picture preview with Edit, Share, and Open Folder, without ever
stealing focus from the application under test.

## Acceptance Criteria

- Behavior in the [approved spec](../../specs/2026-08-14-post-save-preview.md)
  is observable end to end from a hotkey press while Encore is unfocused.
- No existing after-save behavior regresses; the save path itself is never
  blocked or delayed by preview work.
- Every ticket's focused validation and the relevant quality lane pass.

## Context

- [Post-save preview specification](../../specs/2026-08-14-post-save-preview.md)
- [Replay trigger spec](../../specs/2026-08-12-replay-trigger.md) — the
  `saved` state and its emit path are the trigger seam.
- [Settings window spec](../../specs/2026-08-13-settings-window.md) — the
  after-save choice and Saving section this extends.
- [Paper & grain UI migration plan](2026-08-13-paper-grain-ui.md) — shipped
  window, capability, thumbnail, clipboard, and editor-open seams this reuses.
- [Architecture](../../architecture/index.md) · [Quality](../../quality.md)

## Tickets

### PP-01 — Preview payload and the `preview` after-save choice

**Delivered behavior:** A `preview_payload(id)` command returns everything a
preview needs for a saved replay — display name, duration, size, and the
absolute video path — reusing the editor header's metadata reading and the
library's traversal guard. The after-save choice accepts a new `preview`
value that routes through the existing dispatch, and Settings → Saving shows
it as a fourth option. No window yet: the routing is observable through the
emitted state and tests.

**Acceptance criteria:**

- The payload reports real values from the bundle and degrades (omitting
  duration rather than inventing it) when metadata is missing.
- A payload request for an id outside the save destination is rejected with
  a stable error code.
- `preview` persists like the other after-save values and survives relaunch;
  choosing it does not break the existing reveal / open-editor / nothing
  paths.
- The default after-save value becomes `preview` for fresh installs, while
  an already-persisted value is left alone.

**Validation:** Rust tests for payload construction, the guard rejection,
settings round-trip and default change, and dispatch routing; frontend
check/build for the new Settings option. Behavior lane.

**Blocked by:** none.

### PP-02 — Preview window that appears without stealing focus

**Delivered behavior:** A new `preview` window (about 320×250, always-on-top,
undecorated, transparent, paper-surface card) appears in a screen corner
inside the work area when a replay is saved with the `preview` choice, shows
the payload from PP-01 — display name, mono duration · size line, and the
still thumbnail — and can be dismissed with its close control or Escape. It
never takes keyboard focus.

**Acceptance criteria:**

- Pressing the hotkey while another application is focused shows the preview
  and leaves keyboard focus where it was.
- The window sits inside the monitor work area with a margin and does not
  overlap the floating bar.
- Its capability grants only what it uses; no focus-stealing permission.
- A second save replaces the contents of the same window instead of opening
  another one.
- Renders correctly in light and dark themes.

**Validation:** frontend check/build; a macOS smoke of hotkey-while-unfocused
confirming appearance, placement, and that focus never moved; rapid
double-save check. Behavior lane.

**Blocked by:** PP-01.

### PP-03 — Confirmation sound

**Delivered behavior:** A short bundled chime plays on a successful save,
governed by a new "Play a sound when a replay is saved" toggle in Settings →
Saving (default on). Failed saves stay silent.

**Acceptance criteria:**

- The sound plays on save and never on failure; nothing reaches the network.
- The toggle persists across relaunch and silences the sound immediately when
  turned off, leaving the preview itself unaffected.
- The sound plays regardless of which window has focus.

**Validation:** Rust settings round-trip test; manual save-with-sound and
save-with-toggle-off smokes. Behavior lane.

**Blocked by:** PP-01. (Independent of PP-02; sound and window can land in
either order.)

### PP-04 — Edit, Share, and Open Folder actions

**Delivered behavior:** The preview's three buttons act on the shown replay:
Edit opens it in the Editor and dismisses the preview, Open Folder reveals
its bundle in Finder and dismisses, Share copies the replay file reference to
the clipboard and shows an inline "Copied" confirmation while staying open.
All three reuse existing commands.

**Acceptance criteria:**

- Each action targets the exact replay shown, verified against a specific
  saved bundle.
- Share's clipboard result pastes as a file into Finder and chat
  applications; a copy failure surfaces inline instead of silently doing
  nothing.
- Dismiss-on-action applies to Edit and Open Folder only.

**Validation:** frontend check/build; a macOS smoke exercising all three
buttons against one known replay. Behavior lane.

**Blocked by:** PP-02.

### PP-05 — Video preview, auto-dismiss, and motion behavior

**Delivered behavior:** The preview plays a looping muted video of the saved
replay at reduced size, falling back to the still thumbnail and then the
placeholder. It dismisses itself after about eight seconds, pausing that
countdown while the pointer is over it, and stops playback when it
dismisses. Under a reduced-motion preference it shows the still frame and
does not autoplay.

**Acceptance criteria:**

- Video plays for a real saved replay; a missing or unreadable video falls
  back visibly rather than showing a broken media area.
- The preview dismisses on its own after the delay, stays while hovered, and
  resumes the countdown when the pointer leaves.
- Playback stops on dismiss — no audio, no lingering decode.
- Reduced motion suppresses autoplay.

**Validation:** a timing-model unit test for the dismiss/hover behavior;
frontend check/build; a macOS smoke on a real 10-minute replay watching CPU
during preview. Behavior lane.

**Blocked by:** PP-02.

## Dependency Order

PP-01 → PP-02 → PP-04, PP-05
PP-01 → PP-03 (parallel with PP-02)

## Validation Lane

Behavior lane per ticket. The spec's full macOS smoke (hotkey while
unfocused → sound → preview → each action) runs once after PP-05 as the
milestone check for this plan.

## Decision Log

- 2026-08-14: Spec self-grilled and approved; decisions recorded in the
  spec's "Grilled Decisions" section.
- 2026-08-14: Share ships as clipboard copy in v1; the native macOS share
  sheet stays an open question and would be its own ticket.
- 2026-08-14: The preview hangs off the existing after-save dispatch seam
  rather than a new trigger path, so hotkey, bar-button, and retried saves
  behave identically for free.

## Progress Log

- 2026-08-14: Plan created; no tickets started.
