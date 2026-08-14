# Post-Save Preview Plan

Status: **IMPLEMENTED** — PP-01 through PP-05 complete (2026-08-14); every
ticket in this plan has landed. What remains is the plan's milestone check:
the spec's full macOS smoke (hotkey while unfocused → sound → preview →
each action → auto-dismiss), plus the light/dark and reduced-motion visual
checks, none of which can run headless.

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

**Note (found during PP-01 review):** a saved replay has two identifiers.
`SavedReplaySnapshot::id` is a session counter (`replay-1`) understood only
by the in-memory replay state; the on-disk id every filesystem-facing
surface uses is `display_name`, the bundle's folder name. Pass
`display_name` to the preview payload.

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

All five landed in that order (PP-01, PP-02, PP-03, PP-04, PP-05); nothing
in the chain is outstanding.

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
- 2026-08-14: PP-01 complete. New `preview` module (`preview/payload.rs`,
  `preview/commands.rs`) exposes the `preview_payload(id)` command —
  display name reused verbatim from `editor::header` (same title
  convention), total bytes and video path from the same read, duration
  derived from the bundle's evidence window and omitted when
  `metadata.json` records none. Ids go through `library::resolve_bundle_dir`,
  so anything outside the destination is rejected with `library_invalid_id`.
  After-save gained `preview` as a valid value and as the fresh-install
  default; `settings::after_save::sanitized` leaves any already-persisted
  valid value (including the old `nothing` default) untouched, covered by a
  new test. Routing moved into a new `replay/after_save_choice.rs` so the
  choice-to-action mapping is testable without an `AppHandle`; the dispatch
  now matches on that enum and its `Preview` arm is an explicit no-op
  awaiting PP-02's window. Settings → Saving shows "Show preview" first.
  Library command wrappers moved out of `lib.rs` into
  `library/commands.rs` (mirroring `editor/commands.rs`) to make room under
  the 350-line ceiling. Validation: `npm run check`, `npm run build`,
  `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (200 pass, 8 ignored),
  `check-quality-gates.mjs`, `check-sonata.sh` — all green.
- 2026-08-14: PP-02 complete. New `preview` window in `tauri.conf.json`
  (320×250, undecorated, transparent, always-on-top, `skipTaskbar`,
  `visibleOnAllWorkspaces`, `visible: false`, and `focus: false` — the v2
  key for "do not focus on creation"), with its own
  `capabilities/preview.json` granting only `core:default` and
  `core:window:allow-hide`: it is positioned from Rust, so it needs no
  window-positioning permission, and it is deliberately given nothing that
  could take focus. `preview::window` adds a `PreviewContext`
  (`Mutex<Option<PreviewPayload>>`, mirroring `EditorContext`) plus
  `show`, which builds the payload first (a replay that cannot be
  described never reaches the screen), records it, positions the window,
  `show`s it without ever calling `set_focus`, and emits `preview-changed`
  so a second save swaps this one window's contents instead of opening
  another. Placement lives in a pure `preview::placement` so the rule is
  testable without an `AppHandle`: bottom-right of the monitor work area
  with the bar's 16pt margin, lifted clear of a reserved bottom band (the
  measured floating-bar height plus its margin) — a test on a 1024pt-wide
  work area shows the centered 760pt bar reaches into that corner, so
  horizontal separation alone would not hold. The `AfterSaveAction::Preview`
  arm now calls it with `saved.display_name` (the on-disk id), never
  `saved.id`. `preview_context` is the window's bootstrap read.
  `PreviewWindow.svelte` (routed by label through `AppRouter`'s nested
  `{:else}`) renders a paper card with the `library_thumbnail` still (the
  striped placeholder on any failure), the display name, and a mono
  "duration · size" line via the Library's own `formatCardSubline`;
  Escape and the close dot both hide the window so it can be reused.
  Edit/Share/Open Folder stay for PP-04; the card leaves their row space.
  Validation: `npm run check`, `npm run build`, `cargo fmt --check`,
  `cargo clippy -D warnings`, `cargo test` (207 pass, 8 ignored),
  `check-quality-gates.mjs`, `check-sonata.sh` — all green. Not yet
  covered: the macOS hotkey-while-unfocused smoke and the light/dark
  visual check.
- 2026-08-14: PP-03 complete. The chime plays from Rust, not a webview: a
  new `sound` module spawns `afplay <resource>` on a detached thread from
  `after_save_dispatch::apply`, deliberately *outside* the choice `match`,
  so it is heard with every Encore window hidden (menu-bar mode) or
  unfocused, is independent of the after-save choice (`nothing` still
  chimes), and never delays the save. It is reached only through
  `honor_after_save`'s `saved`-state guard, so a failed save stays silent
  by construction, and every failure inside it is swallowed. The asset is
  ours — `src-tauri/resources/save-chime.wav`, a 26.5 KB 0.3s two-tone
  chime (A5 → E6, 44.1 kHz mono 16-bit) generated deterministically by
  `scripts/generate-save-chime.mjs`; no `/System/Library` sound is
  depended on. It ships through `tauri.conf.json`'s `bundle.resources` and
  resolves via `BaseDirectory::Resource`, which answers in both worlds:
  bundled it is `Contents/Resources/resources/save-chime.wav`, and in dev
  `tauri-build` stages the same relative path next to the debug binary
  (verified: `src-tauri/target/debug/resources/save-chime.wav`), with the
  source tree kept as a debug-only last-resort fallback. `save_sound: bool`
  persists with the versioned/corrupt-tolerant pattern; because its default
  is *true* it carries its own `#[serde(default = "default_save_sound")]`
  rather than `#[serde(default)]`, so pre-PP-03 files and corrupt files
  land on the sound being on — covered by tests for the missing field, the
  corrupt file, an off round-trip, and a setter round-trip that preserves
  other persisted fields. `update_save_sound` broadcasts `settings-changed`
  like its siblings. Settings → Saving gains the toggle through a new
  self-contained `SettingsSaveSoundRow.svelte` (existing `.switch` markup),
  which keeps the tracked `SettingsSavingSection.svelte` at an unchanged
  complexity. Validation: `npm run check`, `npm run build`,
  `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (213 pass,
  8 ignored), `check-quality-gates.mjs`, `check-sonata.sh` — all green.
  Not yet covered: the manual save-with-sound / toggle-off macOS smokes.
- 2026-08-14: PP-04 complete. The card's reserved row now holds Edit /
  Share / Open Folder, living in a new `PreviewActions.svelte` rather than
  in `PreviewWindow.svelte` (already tracked by the SCC gate at its PP-02
  complexity of 2, which this leaves unchanged — the parent gained only an
  import and an unconditional child tag). All three target the shown
  payload: `payload.id` is the bundle folder name, exactly the id
  `open_editor_window` and the library commands resolve, and
  `payload.videoPath` sits inside the save destination so it clears
  `copy_export_to_clipboard`'s own `guard_within_destination` check.
  Open Folder needed a new command: `open_export_folder` opens the
  *destination folder* without pointing at any replay, while the spec asks
  for the saved bundle to be revealed. `library::reveal_bundle`
  (`library/reveal.rs`) resolves the id through the same
  `guard::resolve_replay_file` every other library entry point uses
  (`library_invalid_id`), reports `library_replay_missing` for a bundle
  that is not on disk, and otherwise delegates to `replay::reveal_in_finder`
  — promoted from `pub(super)` to `pub(crate)` so the `open -R` invocation
  is not duplicated — failing as `library_reveal_failed`. Exposed as the
  `reveal_replay_bundle` command; its guard rejections and the missing-bundle
  case are covered by tests that never reach Finder. Dismiss-on-action
  applies to Edit and Open Folder *only on success*: a failure that hid the
  preview would take its own error label with it, so failures instead
  surface as a brief inline label (`Editor failed` / `Reveal failed` /
  `Copy failed`, 4s) in the same slot Share's "Copied" confirmation uses
  (2s), absolutely positioned so a 320pt card never reflows mid-interaction.
  Because the window is hidden-and-reused rather than closed, the row never
  remounts, so an `$effect` keyed on the payload clears any lingering
  notice when `preview-changed` swaps the replay in — a re-shown preview
  can never inherit the previous save's "Copied". Styling is tokens only:
  accent pill with `--shadow-accent` for Edit, `--surface-raised` hairline
  pills for the other two, `--health` / `--attention` for the notice.
  **No capability change:** these are application commands reached through
  `invoke`, and Tauri v2 capabilities gate core/plugin permissions, not
  `generate_handler!` commands, so `capabilities/preview.json` still grants
  only `core:default` + `core:window:allow-hide` — in particular nothing
  focus-related was added. Validation: `npm run check`, `npm run build`,
  `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`,
  `check-quality-gates.mjs`, `check-sonata.sh` — all green. Not yet
  covered: the macOS smoke exercising all three buttons against one known
  replay.
- 2026-08-14: PP-05 complete; this plan is now fully implemented. **The
  asset-protocol grant was missing.** `editor::open` grants it
  (`asset_protocol_scope().allow_directory(destination, true)`) but
  PP-02's `preview::show` did not, and `security.assetProtocol.scope` in
  `tauri.conf.json` is empty, so the card's `<video>` would have loaded
  only by accident — after the Editor had been opened at least once this
  session, and never on a fresh launch. `preview::show` now makes the same
  grant (before `show`, failing as `preview_scope_failed`); it is
  process-wide and idempotent, so this widens nothing beyond the resolved
  save destination the Editor already grants. No capability change: the
  asset protocol is gated by config plus that runtime scope, not by a
  window permission, so `capabilities/preview.json` still grants only
  `core:default` + `core:window:allow-hide`. The media area moved into a
  new `PreviewMedia.svelte` holding the whole fallback chain — video →
  `library_thumbnail` still → striped placeholder — so the box is never
  blank: the `<video>`'s `onerror` flips to the still (reset per payload,
  since the row never remounts), and the still's own failure already fell
  through to the placeholder. Auto-dismiss timing lives in a pure
  `previewDismiss.ts`: `advanced(elapsedMs, hovered, tickMs)` is a
  `Record` lookup, not a branch — a hovered tick is simply worth zero, so
  hover can only postpone a dismissal, never cancel one already fired or
  rewind counted time — and `shouldDismiss(elapsedMs)` compares against
  `DISMISS_AFTER_MS = 8000`. **No JavaScript test runner exists in this
  repository** (`package.json` has no vitest/jest and the ticket forbids
  adding one), and the rule is webview timing that would not be honest to
  relocate into Rust, so the spec's "timing-model unit test" is *not*
  covered by an automated test. The mitigation is that the module is
  branch-free, total, has no time source of its own, and carries its
  worked example table in its header; adding vitest and a dozen-line spec
  is the obvious follow-up. The interval and the one dismissal branch sit
  in a render-less `PreviewCountdown.svelte`, restarted whenever the
  payload swaps or the card is shown again (so a re-shown preview never
  inherits a nearly expired clock) and not ticking at all while dismissed.
  Hover is a `pointerenter`/`pointerleave` pair on the card surface, which
  covers the action buttons too, so no click can be overtaken mid-press.
  Playback stops by construction: `showing` gates the `<video>`'s
  existence, so dismissing (or swapping payloads) unmounts it and the
  effect cleanup pauses it, removes `src`, and calls `load()` — no decode
  behind a hidden window. Under `prefers-reduced-motion: reduce` the video
  is never rendered and the still shows instead, while the countdown keeps
  working. `PreviewWindow.svelte` stays at its tracked SCC complexity of 2
  (it lost the `{#if thumbnailUrl}` block and gained only imports,
  branch-free state, and two child tags); the new files measure 4
  (`PreviewMedia`), 2 (`PreviewCountdown`), and 0 (`previewDismiss.ts`),
  and `preview.css` stays at 0. Validation: `npm run check`,
  `npm run build`, `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test` (215 pass, 8 ignored), `check-quality-gates.mjs`,
  `check-sonata.sh` — all green. Not yet covered: the macOS smoke on a
  real 10-minute replay watching CPU during preview, the reduced-motion
  check, and this plan's end-to-end milestone smoke.
