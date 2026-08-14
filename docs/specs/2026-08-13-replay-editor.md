# Replay Editor

> Status: **APPROVED** (self-grilled 2026-08-13; decisions below).
> New surface (no code exists); layout settled by mockup 1d.
> Depends on: [Paper & grain design system](2026-08-13-paper-grain-design-system.md),
> [Replay library](2026-08-13-replay-library.md),
> [Video pipeline](2026-08-12-video-pipeline.md) (keyframe cadence bounds
> what lossless cutting can promise).

## Problem and Outcome

A saved replay usually contains minutes of context around seconds of signal.
Testers need to trim the head/tail and cut dead stretches before handing
evidence to a developer — without a re-encode that would cost time and
quality.

Outcome: an Editor window (mockup 1d) that trims and segments a saved replay
and exports losslessly by default.

## In Scope

- Editor window: back link to Library, replay title (`Today, 4:32 PM`), spec
  line (`1080p · 30 fps · 96 MB`), and a `lossless — no re-encode` badge
  whenever the current edit can be produced without re-encoding.
- Video preview with play/pause and a playhead readout
  (`02:14.6 / 08:47.3 kept` — elapsed over total kept duration).
- Timeline: time ruler; draggable in/out trim handles; excluded head/tail
  rendered dimmed; interior cut regions rendered hatched with a
  `CUT {duration}` chip; a playhead marker.
- Editing actions: `Split at playhead` (S), `Remove segment`, Undo/Redo.
- Export bar: format toggle `MP4` / `GIF`, `Copy to clipboard`, destination
  display (export folder), primary `Export ⌘E` button.
- Export semantics:
  - Export always writes a new file; the original saved replay is never
    modified in place.
  - MP4 export is lossless (container-level cut/concat) when cut points can
    land on keyframes; if a requested edit cannot be lossless, the UI says
    so before export (the badge disappears / a notice appears) rather than
    silently re-encoding — whether frame-accurate re-encode is offered as a
    fallback is a grilling question.
  - GIF export is inherently a re-encode; scope (resolution/fps caps) needs
    grilling.
- Library integration: exported/trimmed results appear in the Library with
  the `TRIMMED` badge; entry points are Library "Open in editor" and the
  after-save `Open editor` setting.

## Out of Scope

- Annotations, zoom/pan, audio (no audio exists in the MVP), speed changes,
  multi-clip timelines, any effect that requires re-encoding video content.
- Modifying rolling-buffer segments; the editor only operates on saved
  evidence bundles.

## Acceptance Criteria

- Opening a saved replay shows its real duration, resolution, fps, and size.
- Trim handles and interior cuts update the kept-duration readout; playback
  skips cut regions.
- Split/remove/undo/redo behave predictably; S and ⌘E hotkeys work while
  the editor window is focused.
- Lossless MP4 export of a trimmed replay completes without re-encoding
  (verifiably: export time is near-instant relative to duration and stream
  parameters are unchanged), plays in QuickTime, and appears in the Library
  as trimmed.
- The lossless badge is truthful: it reflects whether the current edit, as
  specified, will re-encode.
- Copy to clipboard places the exported file on the pasteboard.
- Both themes match the design system.

## Implementation Constraints and Settled Decisions

- Cutting uses the bundled FFmpeg sidecar (stream copy) consistent with the
  existing packaging pipeline; the one-second keyframe interval from the
  video-pipeline spec is the lossless cut granularity.
- Edits are non-destructive until export; closing the editor without
  exporting discards or preserves the edit state per grilling decision.
- Metadata for exported bundles records provenance (source replay id, cut
  list) so evidence remains auditable.

## Expected Validation

- Automated: cut-list → FFmpeg invocation unit tests; probe exported file
  for stream-copy (codec parameters identical, expected duration).
- Manual: trim + interior cut on a real 10-minute replay; QuickTime
  playback; GIF export sanity.

## Grilled Decisions (2026-08-13)

- Lossless-only cutting in v1: trim handles and split points snap to
  keyframes (1 s cadence per the video-pipeline spec), so every MP4 export
  is stream-copy and the lossless badge is always true. No frame-accurate
  re-encode mode; ~1 s granularity is acceptable for QA evidence. The
  snapping is visible in the UI (handles land on tick positions).
- GIF export is in scope but last: downscaled to max 640 px wide at 10 fps
  with two-pass palette generation via the ffmpeg sidecar; the UI states
  that GIF is a re-encode (the lossless badge applies to MP4 only).
- Edit state is per-session only: closing the editor with unexported edits
  asks for confirmation, then discards. No cut-list persistence.
- Copy to clipboard places a file URL reference on the pasteboard (never
  raw video data), pointing at the most recent export — exporting first if
  the current edit has not been exported.
- Export writes a sibling bundle named after the source with a `(trimmed)`
  suffix; its `metadata.json` copies the source metadata plus
  `trimmed: true`, the source replay id, and the applied cut list.

## Risks

- Keyframe cadence deviations in real segments (e.g. after recovery gaps)
  could break clean stream-copy at chosen points; the ffprobe sidecar
  verifies actual keyframe positions rather than assuming 1 s.
- Playback that skips cut regions must stay in sync with the timeline UI;
  covered by manual smoke on a real 10-minute replay.
