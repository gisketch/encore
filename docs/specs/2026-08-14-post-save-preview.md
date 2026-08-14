# Post-Save Replay Preview

> Status: **APPROVED** (self-grilled 2026-08-14; decisions recorded below).
> Extends: [Replay trigger and snapshot](2026-08-12-replay-trigger.md),
> [Settings window](2026-08-13-settings-window.md) (after-save choice),
> [Replay editor](2026-08-13-replay-editor.md),
> [Paper & grain design system](2026-08-13-paper-grain-design-system.md).

## Problem and Outcome

Pressing the replay hotkey today produces almost no feedback. Encore is
usually unfocused when a tester triggers it, so the only signal is a status
line on a floating bar the tester may not be looking at. The tester cannot
tell whether the save worked, cannot see what was captured, and has no way to
act on the replay without hunting for a window or a Finder folder.

Outcome: saving a replay announces itself. A short sound confirms the save,
and a small picture-in-picture preview appears in a screen corner showing the
captured replay with three immediate actions — Edit, Share, Open Folder — then
gets out of the way on its own.

## In Scope

- **Confirmation sound** on a successful save: a short bundled chime, played
  locally, controlled by a Settings toggle (default on). Silent on failure —
  failures keep their existing on-bar error surface.
- **Preview window** (new window label `preview`): small (about 320×250),
  always-on-top, no decorations, transparent, paper-surface card per the
  design system, positioned in a screen corner over the work area.
  - Contents: a looping muted video preview of the saved replay (falling back
    to the still thumbnail, then to the striped placeholder), the replay's
    display name, a mono meta line (duration · size), a close affordance, and
    three action buttons: **Edit**, **Share**, **Open Folder**.
  - Never takes keyboard focus when it appears. Encore must keep working
    while the application under test stays focused.
- **Actions**:
  - *Edit* — opens this replay in the Editor window (the same `editor::open`
    seam the Library's "Open in editor" uses) and dismisses the preview.
  - *Share* — copies the replay file reference to the clipboard so the tester
    can paste it into a chat, ticket, or Finder; shows an inline "Copied"
    confirmation and keeps the preview open.
  - *Open Folder* — reveals the saved bundle in Finder and dismisses.
- **Auto-dismiss** after about 8 seconds, with the countdown paused while the
  pointer is over the preview; Escape and the close affordance dismiss
  immediately. A second save while a preview is showing replaces its contents
  and restarts the countdown rather than opening a second window.
- **Settings**: the after-save choice gains a `preview` option, which becomes
  the new default; a separate "Play a sound when a replay is saved" toggle
  lives in the Saving section.

## Out of Scope

- The native macOS share sheet (AirDrop/Messages/Mail). See the Share
  decision below; this is the most likely first extension.
- System notification-center notifications; the preview is Encore's own
  surface and needs no notification permission.
- Preview for failed or in-progress saves, editing inside the preview, and
  any preview of the rolling buffer before a save.
- Windows behavior.

## Acceptance Criteria

- Triggering a replay by hotkey while another application is focused plays
  the confirmation sound and shows the preview, and the focused application
  keeps keyboard focus throughout.
- The preview shows the replay that was just saved: its display name, its
  duration and size, and moving video from that file (or a still frame, or
  the placeholder — never a broken or empty media area).
- Edit opens that exact replay in the Editor; Open Folder reveals that exact
  bundle in Finder; Share puts a file reference on the clipboard that pastes
  as a file into Finder and chat applications.
- The preview disappears on its own after the dismiss delay, stays while the
  pointer rests on it, and closes immediately on Escape or the close control.
- Saving twice in quick succession leaves exactly one preview window showing
  the most recent replay.
- With the sound toggle off, saving is silent; with the after-save choice set
  to any other value, no preview appears and the previously specified
  behavior for that choice is unchanged.
- A save that fails produces no sound and no preview; the bar reports the
  failure as it does today.
- The preview renders correctly in light and dark themes and honors the
  reduced-motion preference (no video autoplay loop under reduced motion —
  show the still frame instead).

## Implementation Constraints and Settled Decisions

- The preview is triggered from the existing after-save seam that already
  fires when a replay reaches the `saved` state, so hotkey saves, bar-button
  saves, and retried saves all behave identically. No new trigger path.
- Local-only: the sound asset ships in the bundle; nothing reaches the
  network, matching the project's local-only constraint.
- The preview window reuses the existing per-window capability model. It
  needs only what it uses; it must not receive focus-stealing permissions.
- The preview's video source uses the same asset-protocol grant the Editor
  already establishes for the export destination.
- Positioning follows the floating bar's convention of sitting inside the
  monitor work area with a margin, and must not overlap the floating bar.

## Grilled Decisions (2026-08-14)

- **Share means clipboard copy in v1.** The existing guarded clipboard
  command already places a file reference on the pasteboard, which covers the
  real QA need: paste the evidence into a ticket or a chat. A native share
  sheet requires Objective-C interop the codebase has so far avoided. The
  button keeps the name "Share" with a tooltip naming the actual behavior.
  This is the decision most worth revisiting — flagged as an open question.
- **`preview` becomes the default after-save choice**, replacing `nothing`.
  This is the feature's point: saving should announce itself. Existing
  installations keep whatever value they have already persisted.
- **The sound is a separate toggle, not part of the after-save choice**, so a
  tester who wants a silent workspace can keep the visual preview.
- **Video preview over a static thumbnail.** Motion is what tells the tester
  they captured the right moment; the thumbnail path already exists as the
  fallback and costs nothing to reuse.
- **The preview does not block or delay the save.** It renders from the
  already-published bundle; any failure to build the preview leaves the save
  and its recorded state untouched.

## Expected Validation

- A macOS smoke with another application focused: press the hotkey, confirm
  sound, preview appearance, that focus never moved, and that each of the
  three actions drives the right target.
- Automated coverage at the seams that can be tested without a live window:
  the after-save choice routing to the preview path, the preview payload
  built for a saved replay (name, duration, size, video path), rejection of a
  payload request for an id outside the destination, and the auto-dismiss
  timing model.
- A rapid double-save check confirming a single preview window survives.
- Light and dark visual check plus a reduced-motion check.

## Risks and Open Questions

- **Open question — Share behavior.** Clipboard copy in v1; a native macOS
  share sheet is the obvious upgrade and would change this button's contract.
- An always-on-top preview can cover the application under test at the moment
  the tester is still working; corner placement and the short dismiss delay
  are the mitigation, and the delay may need tuning after real use.
- Autoplaying video for a 10-minute 1080p replay must not cost noticeable
  CPU; the preview should play at a reduced size and stop playback as soon as
  it dismisses.
- Sound at an inappropriate moment (screen share, quiet room) is why the
  toggle exists; the default remains on, matching the feature's intent.
