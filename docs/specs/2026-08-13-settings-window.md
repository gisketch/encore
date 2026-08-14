# Settings Window

> Status: **APPROVED** (self-grilled 2026-08-13; decisions below).
> Layout settled by mockup 2a.
> Depends on: [Paper & grain design system](2026-08-13-paper-grain-design-system.md).
> Related: [Always-on lifecycle](2026-08-12-always-on-lifecycle.md) (persistence
> and menu-bar questions are shared; resolve once, in one place).

## Problem and Outcome

Today the only settings surface is the rail itself (source picker, 5m/10m
toggle); nothing persists explicitly and there is no place for destination,
hotkeys, or appearance. The redesign moves configuration into a dedicated
Settings window (mockup 2a) so the bar can stay minimal.

Outcome: a Settings window covering recording defaults, saving behavior,
hotkeys, and general app behavior, persisted locally and honored on relaunch.

## In Scope

Sections and controls, per mockup 2a:

- **Recording**
  - Replay window: dropdown of retention durations (today 5 / 10 minutes;
    the mockup shows "10 minutes" — whether the list grows beyond 5/10 is a
    grilling question). Drives the existing retention setting.
  - Default source: which display/window Encore captures at launch.
- **Saving**
  - Save replays to: shows current destination (default
    `~/Movies/Encore`), Change… opens a native folder picker.
  - After saving: one of `Open editor` / `Show in Finder` / `Nothing`
    (segmented control). `Open editor` depends on the Editor spec; until it
    ships the option is hidden or disabled.
- **Hotkeys**
  - Save replay (default ⌘⌥R), Pause capture (default ⌘⌥P), Open library
    (default ⌘⌥L). Each shows the current chord with an Edit affordance.
    Conflicts/registration failures surface honestly (the replay snapshot
    already models shortcut registration errors).
- **General**
  - Start at login (toggle).
  - Show in menu bar (toggle): hide the floating bar, keep a menu bar icon.
  - Appearance: Light / Dark / System segmented control (feeds the design
    system's theme resolution).

Persistence:

- All settings persist locally (no cloud) and are restored on launch.
- The UI reflects the native state machine's actual values — a setting that
  fails to apply (e.g. shortcut conflict, missing folder) shows the failure
  rather than pretending.

## Out of Scope

- Accent color / grain intensity controls (design-system spec lists them as
  future).
- Telemetry, update settings, network anything.
- Windows-specific settings.

## Acceptance Criteria

- Settings opens from the expanded action bar; a separate window styled per
  the design system, both themes.
- Changing replay window, default source, destination, after-save behavior,
  hotkeys, start-at-login, menu-bar mode, and appearance each takes effect
  immediately where meaningful and survives quit + relaunch.
- Retention change is reflected in the action bar sub-line and in actual
  buffer pruning (existing `set_retention_minutes` behavior).
- Destination change affects the next saved replay; an invalid/missing
  destination is surfaced as an actionable error at save time.
- Hotkey edit rejects unregisterable chords with a visible reason and keeps
  the previous binding.
- With "Show in menu bar" on, the floating bar hides and every bar action
  (save, pause, library, settings, quit) remains reachable from the menu bar
  item.

## Implementation Constraints and Settled Decisions

- Settings storage is a local file owned by the Rust core (single writer);
  the frontend reads via commands/events, never writes directly.
- Defaults match today's behavior: 10-minute window, main display,
  `~/Movies/Encore`, ⌘⌥R, no login item, floating bar visible, System
  appearance.
- Hotkey capture UI must not globally register a chord until confirmed.

## Expected Validation

- Relaunch round-trip test for every persisted setting.
- Shortcut-conflict manual test (bind a chord already taken).
- Menu-bar mode smoke test: hide bar, exercise all actions, restore.

## Grilled Decisions (2026-08-13)

- Retention options stay 5 / 10 minutes only — the backend contract is typed
  to `5 | 10` and the disk budget (~250 MB per 10 min) is calibrated to it.
  Expanding the list is future work with its own disk-budget review.
- Storage: `settings.json` in the app config directory, owned by a new Rust
  `settings` module (single writer, atomic write-and-rename). Frontend
  reads via a `settings_snapshot` command and an `settings-changed` event;
  writes go through an `update_settings` command that applies the change to
  the live system first and persists only what actually took effect.
- Start at login: DEFERRED — the always-on lifecycle grill (2026-08-13)
  deferred launch-at-login and onboarding consent. The row ships hidden;
  when un-deferred it uses the Tauri autostart plugin, defaults OFF, and
  reflects the real login-item registration state.
- Default source: applies at launch only. Live source switching stays in
  the action bar. If the configured source is missing at launch, fall back
  to the main display and surface the substitution in the status line.
- Menu-bar mode v1: a tray icon with a native menu — Save Replay,
  Pause/Resume, Open Library, Settings…, Show Floating Bar, Quit. Toggling
  the setting hides/shows the main window; the tray icon is present in
  both modes only while "Show in menu bar" is on.
- Hotkey editing: a recorder field captures the chord locally, registration
  is attempted only on confirm, and failure (conflict/invalid) restores
  the previous binding with a visible reason. Pause hotkey (⌘⌥P) depends on
  the pause commands from the action-bar spec.
- After-saving options ship as `Show in Finder` / `Nothing` (default
  `Nothing`, matching today's manual reveal); `Open editor` is added by the
  Editor spec's integration ticket and becomes the recommended default
  only after the editor exists.
- Appearance persists in `settings.json` and feeds the design-system theme
  attribute at startup and on change.

## Risks

- Login-item and global-shortcut registration can both fail silently at the
  OS level; every settings row must render the live registration state, not
  the requested one.
- Two windows (bar + settings) sharing one snapshot/event stream is new;
  events must be broadcast to all windows.
