# Paper & Grain UI Migration Plan

Status: **READY** — specs approved (self-grilled 2026-08-13), tickets below.

## Goal

Migrate every Encore surface to the approved paper & grain design system and
ship the three new surfaces (Settings, Library, Editor) it introduces, while
preserving every capture/replay state the current rail reports.

## Acceptance Criteria

- Behavior in the five approved specs is observable at the bar, settings,
  library, and editor seams, in light and dark themes.
- No existing permission, capture, or replay state loses visibility at any
  point in the migration; the bar stays shippable after every ticket.
- Every ticket's focused validation and the relevant quality lane pass.

## Context

- [Design system spec](../../specs/2026-08-13-paper-grain-design-system.md)
- [Action bar spec](../../specs/2026-08-13-action-bar-redesign.md)
- [Settings window spec](../../specs/2026-08-13-settings-window.md)
- [Replay library spec](../../specs/2026-08-13-replay-library.md)
- [Replay editor spec](../../specs/2026-08-13-replay-editor.md)
- [Always-on lifecycle plan](2026-08-13-always-on-lifecycle.md) — delivers
  settings persistence (AL-01) and pause/resume (AL-02) that tickets here
  build on. Do not duplicate that backend work.
- [Architecture](../../architecture/index.md) · [Quality](../../quality.md)

## Tickets

### PG-01 — Token layer, fonts, and theme switching

**Delivered behavior:** One `theme.css` token layer (paper light + charcoal
dark palettes, bundled Instrument Sans / Spline Sans Mono with OFL license
files, grain data-URI, radii/shadow/motion tokens) loaded before component
styles; the existing rail consumes tokens instead of the legacy palette and
renders in paper light, warm-charcoal dark, and system-following modes via a
root `data-theme` attribute.

**Acceptance criteria:**

- All colors, fonts, and shadows in the shell come from `var(--…)` tokens;
  the legacy variables (`--rail`, `--signal`, `--mint`, `--coral`, `--fog`,
  `--rail-raised`, `--line`) are deleted.
- No network font or asset request at runtime.
- Removing `data-theme` follows `prefers-color-scheme` live; setting
  `light`/`dark` overrides it.
- Grain tiles on surface elements and does not shimmer while dragging.
- Reduced-motion preference still suppresses pulse/entrance animation.

**Validation:** frontend check/build; grep gate that legacy variables are
gone; manual light/dark/system visual smoke against mockups. Behavior lane.

**Blocked by:** none.

### PG-02 — Action bar collapsed + expanded layout

**Delivered behavior:** The rail becomes the mockup action bar: collapsed
pill (grip, buffer mark, status cluster with `last {N} min · {source}`
sub-line, library button, accent `Save Replay ⌘⌥R`, chevron) and an expanded
second row (source picker, retention 5m/10m toggle as a temporary tenant
until PG-05, buffer badge from `retention.retainedBytes`, Quit). The window
switches between two fixed logical sizes on expand/collapse.

**Acceptance criteria:**

- Every current state remains reachable and labeled: permission flows swap
  the primary slot (Enable / Settings / Restart / Retry) exactly as the
  current `RailActions` logic does; replay preparing/saved/failed and
  shortcut errors render in the status cluster.
- Library button reveals the export folder in Finder (interim target).
- `tauri.conf.json` drops `maxHeight`; expand/collapse resizes between the
  two sizes without clipping; expansion resets to collapsed on launch.
- Drag region, entrance animation, and reduced-motion behavior preserved.
- Recording pulse animates only while capturing.

**Validation:** frontend check/build; manual walkthrough of each
permission/capture/replay state via the preview snapshot; drag + hotkey
smoke; light/dark visual check against mockups 1b/2b. Behavior lane.

**Blocked by:** PG-01.

### PG-03 — Pause and resume in the bar

**Delivered behavior:** The expanded row gains Pause/Resume wired to the
AL-02 pause commands; the status cluster reports `paused` honestly and Save
Replay stays available while paused.

**Acceptance criteria:**

- Pause → status shows Paused (attention tone, no pulse); Resume restores
  capturing; the button label/icon reflects the actual state machine state.
- Save Replay remains enabled while paused when retained segments exist.
- Pause failure surfaces an error code in the status line.

**Validation:** frontend check/build; manual pause → save → resume smoke.
Behavior lane.

**Blocked by:** PG-02, AL-02.

### PG-04 — Settings window shell + appearance

**Delivered behavior:** A second Tauri window (styled per design system,
titled Settings) opens from the expanded bar's Settings button, hosting the
General → Appearance control (Light/Dark/System) persisted in the AL-01
settings document and applied to all open windows immediately.

**Acceptance criteria:**

- Settings opens/focuses from the bar; closing it never affects capture.
- Appearance choice persists across relaunch and restyles the bar and the
  settings window without reload; System tracks the OS live.
- Settings document gains a versioned `appearance` field; snapshot/event
  plumbing (`settings_snapshot`, `settings-changed`) reaches all windows.

**Validation:** Rust round-trip test for the new field; relaunch smoke;
frontend check/build. Behavior lane.

**Blocked by:** PG-01, AL-01. (PG-02 supplies the button; until then a dev
entry point suffices.)

### PG-05 — Recording section: replay window + default source

**Delivered behavior:** Settings → Recording offers the replay window
dropdown (5 / 10 minutes, wired to `set_retention_minutes` and persisted via
AL-01) and the default launch source; the temporary retention toggle leaves
the bar.

**Acceptance criteria:**

- Changing the replay window updates pruning immediately and the bar
  sub-line (`last {N} min`); it survives relaunch.
- Default source applies at next launch; a missing source falls back to the
  main display with a visible notice (AL-01 behavior).
- Retention control no longer renders in the bar; the bar remains the only
  live source switcher.

**Validation:** frontend check/build; relaunch smoke for both settings;
manual retention-change → sub-line check. Behavior lane.

**Blocked by:** PG-02, PG-04.

### PG-06 — Saving section: destination + after-save behavior

**Delivered behavior:** Settings → Saving shows the export destination with
a native folder picker (Change…) and an after-save segmented control
(`Show in Finder` / `Nothing`, default `Nothing`), both persisted; the
replay pipeline honors them on the next save.

**Acceptance criteria:**

- Destination change affects the next saved replay; an invalid destination
  fails the save with the existing actionable error path, not silently.
- `Show in Finder` reveals the bundle after packaging completes; `Nothing`
  keeps today's manual-reveal behavior.
- Both values survive relaunch; the mockup's `Open editor` option is
  deliberately absent until PG-15.

**Validation:** Rust test for destination persistence + fallback; manual
save-to-changed-folder smoke. Behavior lane.

**Blocked by:** PG-04.

### PG-07 — Hotkeys section with safe rebinding

**Delivered behavior:** Settings → Hotkeys lists Save replay (⌘⌥R), Pause
capture (⌘⌥P), Open library (⌘⌥L) with a recorder-style Edit flow that
registers a chord only on confirm and rolls back visibly on failure.

**Acceptance criteria:**

- Recording a chord never globally registers it before confirm; conflicts
  or registration failures restore the previous binding and show a reason.
- Rebound Save replay works globally while Encore is unfocused; bindings
  persist across relaunch.
- Pause and Open library hotkeys invoke the same paths as their buttons;
  the pause row is disabled until AL-02 lands if sequenced earlier.

**Validation:** Rust persistence test; manual conflict test (bind a taken
chord); unfocused-hotkey smoke. Behavior lane.

**Blocked by:** PG-04; pause row additionally AL-02.

### PG-08 — Menu bar mode

**Delivered behavior:** Settings → General "Show in menu bar" toggles a tray
icon whose menu offers Save Replay, Pause/Resume, Open Library, Settings…,
Show Floating Bar, Quit; enabling it hides the floating bar, and every bar
action stays reachable.

**Acceptance criteria:**

- Toggle on: bar hides, tray appears; toggle off: bar returns, tray leaves.
- Each menu item drives the same command as its bar counterpart, with
  pause/resume reflecting live state.
- Mode persists across relaunch; consistent with the lifecycle decision
  that closing hides rather than quits.

**Validation:** manual full-menu exercise in tray mode + relaunch smoke.
Behavior lane.

**Blocked by:** PG-04, PG-03.

### PG-09 — Start at login (DEFERRED)

Deferred by the always-on lifecycle grill (launch-at-login + consent). When
un-deferred: Tauri autostart plugin, default OFF, toggle reflects real
registration state. **Blocked by:** PG-04 and an explicit un-defer decision.

### PG-10 — Library window: browse saved replays

**Delivered behavior:** A Library window (mockups 1c/2c) opens from the bar
button and ⌘⌥L, listing evidence bundles from the export folder grouped by
day (Today / Yesterday / dated chips, older days behind Show), with count
and size rollups, `Open Folder ↗`, and cards (placeholder thumbnail,
duration, size, saved time) whose primary action opens the MP4 in the system
player. The index rescans on window focus.

**Acceptance criteria:**

- Every bundle in the export folder appears, newest-first, correctly
  grouped; counts/sizes accurate; corrupt or missing `metadata.json`
  renders a degraded card, never a crash.
- Files added/removed externally reconcile on focus.
- Bar library button and ⌘⌥L now open this window (replacing the Finder
  interim target).
- Both themes match the mockups.

**Validation:** Rust tests for scan/grouping over a fixture folder
(including corrupt metadata); manual save → appears-under-Today smoke.
Behavior lane.

**Blocked by:** PG-01, PG-04 (multi-window plumbing pattern); button rewire
touches PG-02.

### PG-11 — Library thumbnails

**Delivered behavior:** Cards show real first-keyframe (≥1 s) JPEG
thumbnails generated lazily by the ffmpeg sidecar into the app cache
directory, keyed by bundle path + size + mtime.

**Acceptance criteria:**

- Thumbnails appear without blocking the list render; regenerated when the
  key changes; never written into the export folder.
- A failed extraction falls back to the styled placeholder permanently for
  that key (no retry loop).

**Validation:** Rust test for cache keying; manual smoke including one
corrupt video fixture. Behavior lane.

**Blocked by:** PG-10.

### PG-12 — Library search and delete

**Delivered behavior:** The search field filters cards by display name and
day-group text as you type; a delete affordance moves the whole bundle
folder to the macOS Trash after in-app confirmation.

**Acceptance criteria:**

- Search narrows visibly and clears cleanly; rollup counts reflect the
  filter.
- Delete removes the bundle from disk (recoverable in Trash) and the list;
  failures are surfaced; the confirmation names the replay.

**Validation:** Rust test for trash-delete path (fixture dir); manual
delete → Trash inspection. Behavior lane.

**Blocked by:** PG-10.

### PG-13 — Editor window: open, play, trim

**Delivered behavior:** `Open in editor` (Library) opens the Editor window
(mockup 1d) on a saved replay: metadata header, `lossless — no re-encode`
badge, video preview with play/pause, playhead readout
(`elapsed / kept`), and keyframe-snapped in/out trim handles with dimmed
excluded regions. Keyframe positions come from ffprobe, not assumption.

**Acceptance criteria:**

- Header shows real duration, resolution, fps, size from the bundle.
- Trim handles snap to actual keyframes; kept-duration readout updates;
  playback honors the trim.
- Library card hover primary action becomes `Open in editor`.

**Validation:** Rust test for keyframe probing; manual trim/playback smoke
on a real replay. Behavior lane.

**Blocked by:** PG-10.

### PG-14 — Cuts, undo/redo, lossless export

**Delivered behavior:** `Split at playhead` (S), `Remove segment`,
Undo/Redo, and `Export ⌘E` producing a stream-copied MP4 bundle named with a
`(trimmed)` suffix whose `metadata.json` carries `trimmed: true`, the source
replay id, and the cut list; the Library shows its `TRIMMED` badge.
Closing with unexported edits asks for confirmation, then discards.

**Acceptance criteria:**

- Playback skips cut regions in sync with the hatched timeline chips.
- Export is verifiably stream-copy (codec parameters unchanged, near-
  instant relative to duration), plays in QuickTime, appears in the
  Library as trimmed; the source bundle is untouched.
- Undo/redo round-trips every edit operation.

**Validation:** Rust tests for cut-list → ffmpeg invocation and exported
metadata; ffprobe assertion of stream copy; manual 10-minute-replay smoke.
Critical lane (touches evidence integrity).

**Blocked by:** PG-13.

### PG-15 — GIF export, clipboard, editor integrations

**Delivered behavior:** The export bar's GIF toggle (≤640 px wide, 10 fps,
two-pass palette, clearly marked as re-encode), `Copy to clipboard` (file
URL of the latest export, exporting first if needed), and the deferred
integrations: after-save `Open editor` option (recommended default once
present) and the ⌘E/S hotkeys verified window-scoped.

**Acceptance criteria:**

- GIF of a trimmed range plays in Safari/Slack preview at bounded size;
  lossless badge does not apply to GIF and the UI says so.
- Copy to clipboard pastes a playable file into Finder/Slack.
- After-save `Open editor` opens the just-saved replay in the editor.

**Validation:** manual GIF + clipboard + after-save smokes; frontend
check/build. Behavior lane.

**Blocked by:** PG-14, PG-06.

## Dependency Order

PG-01 → PG-02 → PG-03 (needs AL-02)
PG-01/AL-01 → PG-04 → PG-05, PG-06, PG-07, PG-08 (needs PG-03)
PG-04 → PG-10 → PG-11, PG-12, PG-13 → PG-14 → PG-15
PG-09 deferred.

## Validation Lane

Per-ticket lanes as listed; PG-14 runs the critical lane. Run the milestone
lane before declaring the migration complete.

## Decision Log

- 2026-08-13: Specs self-grilled and approved; decisions recorded in each
  spec's "Grilled Decisions" section.
- 2026-08-13: Pause backend and settings persistence are owned by the
  always-on lifecycle plan (AL-01/AL-02); this plan only consumes them.
- 2026-08-13: Start-at-login deferred, matching the lifecycle grill.
- 2026-08-13: Retention toggle stays in the expanded bar until PG-05
  removes it (expand → migrate → contract), so the control never vanishes.
- 2026-08-13: User decision — the accent token is green `#7a9b6d`, not the
  mockup's default orange `#dd7a55`. PG-01 ships the green token; visual
  smokes compare structure and tints against the mockups with this
  substitution in mind.

## Progress Log

- 2026-08-13: Plan created.
- 2026-08-13: PG-01 and PG-02 implemented and reviewed. Validation: svelte-
  check/build clean, clippy clean, 64 Rust tests pass, SCC and harness gates
  pass, visual smoke of collapsed/expanded in light and dark. Review fixes:
  recording pulse now animates only in the `capturing` state, the saved-
  state Open button uses the quiet style, and window resizes are serialized
  against rapid toggles. Deliberate deferrals re-confirmed: Pause button →
  PG-03, Settings button → PG-04, retention control leaves the bar in
  PG-05.
- 2026-08-13: PG-04 implemented. A hidden `settings` window (560x320,
  decorations off, transparent, not always-on-top) is declared in
  `tauri.conf.json` and shown/focused by `open_settings_window`; the
  expanded bar's new Settings pill (ring glyph, between Pause and the
  spacer) calls it. `SettingsDocument` gains a sanitized `appearance`
  field (default `system`); `settings_snapshot`/`update_appearance`
  read/write it through `CaptureService`'s single settings writer and
  `update_appearance` broadcasts `settings-changed` via `app.emit` to every
  window. One Vite entry now routes on `getCurrentWindow().label` (new
  `AppRouter.svelte`): "settings" renders `SettingsWindow.svelte`, anything
  else renders `CaptureShell`. Both windows fetch the snapshot at startup
  and listen for `settings-changed`, applying it through
  `appearance.ts#applyAppearance` (data-theme set/removed on
  `document.documentElement`). Validation: cargo fmt/clippy/test (91 Rust
  tests, 4 new covering round-trip, missing-field default, and
  corrupt-value tolerance), svelte-check/build clean, SCC and sonata gates
  pass.
- 2026-08-13: PG-05 implemented. Settings gains a Recording section (above
  General): "Replay window" (5/10 min pill select) calls the existing
  `set_retention_minutes` — untouched, so the bar sub-line keeps reflecting
  it via `capture-state-changed` as before — and "Default source" (pill
  select over `list_capture_sources`) calls a new `update_default_source`
  command that resolves the id and persists it as the launch target without
  touching live capture. On the Rust side, `CaptureService`'s old
  `startup_target` field became `default_target: RwLock<PersistedTarget>`,
  a small independently-cached slice (mirroring the `appearance` pattern)
  that both the bar's live source switch (`persistence::persist`) and the
  new Settings path (`default_source::set_default_source_by_id`) write to,
  last write wins, matching the spec's "default source applies at next
  launch; bar stays the only live switcher." `SettingsSnapshot` grew
  `retention_minutes` and `default_target` fields so the Settings window can
  show the current values on open. The retention 5m/10m toggle and its
  `onSetRetention` plumbing are removed from `BarAdvancedRow`/`CaptureShell`
  (and the now-dead `.retention` CSS rule from `app.css`), leaving
  pause/settings/buffer/quit in the advanced row per PG-05's contract.
  New frontend files (`recordingSettings.ts`, `SettingsRecordingSection.svelte`)
  keep the source-matching branching out of the SCC-gated tracked files.
  Validation: cargo fmt/clippy/test (95 Rust tests, 4 new covering the
  default-source persistence path — round trip, cache sync, field
  preservation, unknown-id tolerance), svelte-check/build clean, SCC and
  sonata gates pass.
