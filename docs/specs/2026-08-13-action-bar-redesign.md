# Action Bar Redesign (Collapsed + Expanded)

> Status: **APPROVED** (self-grilled 2026-08-13; decisions below).
> Layout is settled by mockups 1b (light) and 2b (dark).
> Depends on: [Paper & grain design system](2026-08-13-paper-grain-design-system.md).

## Problem and Outcome

The current floating rail shows every control at once (status, source picker,
5m/10m retention toggle, replay action, local badge) in the legacy dark-glass
style. The approved direction is a calmer two-state pill: a collapsed bar with
only identity, status, and the primary action; advanced controls behind an
expand chevron. Retention moves out of the bar entirely (into Settings, per
mockup turn 2 note "retention moved out of the bar").

Outcome: the floating shell becomes the mockup action bar, preserving every
existing capture/replay state the rail surfaces today.

## In Scope

Collapsed bar (default, ~620×60 pill):

- Drag grip (Tauri drag region, as today).
- Encore buffer mark (three-bar glyph on an accent tint chip).
- Status cluster: pulsing recording dot + primary label + mono sub-line
  `last {N} min · {source label}` (e.g. `last 10 min · Built-in Display`).
- Library button (circular, grid glyph) — opens the Library window.
- Primary accent button `Save Replay ⌘⌥R`.
- Expand/collapse chevron (circular).

Expanded bar adds a second row:

- Source picker (display/window, same command surface as today's
  `switch_capture_source` + `list_capture_sources`).
- Pause / Resume capture button.
- Settings button — opens the Settings window.
- Buffer/local badge: `● buffer {size} · local` (live retained-bytes from the
  capture snapshot; replaces today's standalone "Local" badge).
- Quit button (text, quiet styling).

State mapping — every state the rail handles today must remain observable:

- Permission flow: `permission_required` → Enable, `permission_denied` →
  Settings (TCC), `restart_required` → Restart, capture `failed` /
  `source_unavailable` → Retry. These replace the primary action slot with
  the recovery action, as the current `RailActions` logic does.
- Capture states (`stopped/starting/capturing/paused/recovering/…`) drive the
  status label and dot tone; the pulse animation runs only while recording.
- Replay states: `preparing` (progress), `saved` (confirmation + reveal
  affordance), `failed` (retry) keep their current behaviors under the new
  styling. Shortcut-registration failure remains visible in the status line.

## Out of Scope

- Retention duration control in the bar (moves to Settings spec).
- The Library, Settings, and Editor surfaces themselves.
- Menu-bar-only mode (Settings spec / always-on lifecycle).
- New backend capture capabilities; `pause` may reuse an existing state if
  the capture state machine already models it.

## Acceptance Criteria

- Bar renders collapsed by default; chevron toggles expanded state; the
  window resizes (or hosts both rows) without clipping, in light and dark.
- All permission/recovery flows reachable and labeled as today; no state that
  the current rail reports becomes invisible.
- Save Replay triggers the existing `trigger_replay` flow; hotkey hint shown.
- Pause toggles capture and the status reflects `paused` honestly (UI never
  claims capture while the native state machine says otherwise).
- Buffer badge shows live retained size; Library/Settings buttons open their
  windows (or a stub target until those specs land).
- Dragging, entrance animation, and reduced-motion behavior preserved.

## Implementation Constraints and Settled Decisions

- Keep the existing snapshot/event contract (`capture-state-changed`,
  `replay-state-changed`); this is a presentation-layer migration.
- Retention remains a backend setting (5/10 min) — only its control surface
  moves; the sub-line reflects the configured window.
- Expanded/collapsed is UI state only; it does not persist across launches
  unless grilling decides otherwise.

## Expected Validation

- Manual walkthrough of each permission/capture/replay state (existing
  preview snapshot mechanism can drive non-native states).
- Light/dark visual check against mockups 1b and 2b.
- Hotkey and drag smoke test on macOS.

## Grilled Decisions (2026-08-13)

- Pause is new backend work: the capture state machine already models a
  `paused` state (used for blank-frame streams) but exposes no user command.
  Add `pause_capture` / `resume_capture` commands that stop frame delivery
  and segment appending while retaining the existing rolling buffer, report
  `paused` honestly in the snapshot, and count the gap in
  `evidenceGapCount`. Save Replay stays available while paused (the buffer
  still holds evidence).
- Window sizing: keep `resizable: false` and drop the current
  `maxHeight: 104`; the frontend calls the window `setSize` API to switch
  between two fixed logical sizes — collapsed (~760×84) and expanded
  (~760×144, second row + shadow padding). Expansion state is UI-only and
  resets to collapsed on launch.
- Library button interim target: until the Library window ships, the button
  reveals the export folder in Finder (existing reveal machinery). The
  Library ticket rewires it; the button is never hidden.
- Transitional deviations (recorded in the PG plan, PG-02): the retention
  5m/10m control stays in the expanded row until the Settings window's
  Recording section (PG-05) takes it over, so the control never vanishes;
  the Settings button and Pause button appear with PG-04 and PG-03
  respectively rather than shipping as dead controls in PG-02.
- Buffer badge reads `retention.retainedBytes` from the existing snapshot;
  no new backend surface.

## Risks

- Programmatic resize of a transparent always-on-top window may flicker on
  macOS; if it does, fall back to a fixed-height window sized for the
  expanded bar with transparent slack (visual result identical).
- Pause semantics interact with sleep/wake recovery paths in the capture
  monitor; state-machine tests must cover pause → sleep → wake → resume.
