# Menu Bar as the Primary Control Surface

> Status: **APPROVED** (self-grilled 2026-08-14; decisions below).
> Supersedes the optional "Show in menu bar" mode shipped in the
> [paper & grain plan](../exec-plans/active/2026-08-13-paper-grain-ui.md)
> (PG-08). Extends: [Always-on lifecycle](2026-08-12-always-on-lifecycle.md),
> [Settings window](2026-08-13-settings-window.md),
> [Action bar redesign](2026-08-13-action-bar-redesign.md).

## Problem and Outcome

Encore currently has two competing control surfaces whose relationship is a
persisted setting. With "Show in menu bar" off, the menu bar offers only
Show/Hide/Pause/Quit and the floating bar carries the real controls; with it
on, the bar disappears and the menu grows. A tester has to know which mode
they are in to know where a control lives, and the floating bar — a window
that sits over the application under test — is treated as the primary home
for actions it should not need to own.

Outcome: the menu bar is Encore's permanent, complete control surface. The
floating bar becomes an optional convenience window that is shown at launch,
hidden by closing it, and brought back from the menu bar. Neither surface is
a mode; both are always available in the same way.

## In Scope

- **Launch**: Encore starts with the floating action bar visible, exactly as
  it does today when menu-bar mode is off.
- **Closing the bar hides it.** Capture continues untouched, the menu bar
  icon remains, and nothing about the save pipeline changes. This is already
  the close behavior; it becomes the documented, only behavior.
- **The menu bar always carries every action**, regardless of whether the bar
  is visible:
  - Save Replay
  - Pause Capture / Resume Capture, labeled from live capture state
  - Open Library
  - Settings…
  - Show Action Bar (brings the floating bar back and focuses it)
  - Quit Encore
- **Double-clicking the menu bar icon shows the action bar**, as a shortcut
  for the menu's "Show Action Bar" item.
- **Removal of the "Show in menu bar" setting** and its persisted value. The
  menu bar is no longer a mode, so the toggle has nothing left to mean. A
  settings document that still records the old value loads without error and
  the value is ignored.
- Every menu action drives the same code path as its floating-bar and hotkey
  counterpart, so the three surfaces cannot drift.

## Out of Scope

- Launch-at-login, still deferred by the lifecycle spec.
- A Dock icon or app-switcher presence; Encore remains a menu-bar accessory.
- Changing the floating bar's own layout, the hotkeys, or any capture,
  save, library, editor, or preview behavior.
- Menu bar status text or an icon that changes with capture state.

## Acceptance Criteria

- A fresh launch shows the floating action bar and a menu bar icon.
- Closing the floating bar hides it: capture keeps running, the replay
  hotkey still saves, and the menu bar icon stays.
- With the bar hidden, every action above is reachable from the menu bar and
  performs the same thing its floating-bar counterpart does.
- The Pause/Resume item reflects the live capture state each time the menu is
  opened, and toggling it is visible in the floating bar when the bar is
  shown.
- "Show Action Bar" restores and focuses the floating bar from any state.
- Double-clicking the menu bar icon shows the floating action bar.
- Settings no longer offers "Show in menu bar", and a settings file written
  by a previous version still loads with its other values intact.
- Quit from the menu bar is the only path that ends the process.

## Implementation Constraints and Settled Decisions

- The menu is built from one pure description of "which actions, in what
  order, for this capture state" so its shape stays testable without a live
  app — the existing menu model, with the mode parameter removed.
- Menu items route through the shared action paths the hotkeys already use;
  no menu item may reimplement an action.
- Removing the persisted setting must be tolerant, not destructive: an older
  settings document keeps loading, and no other persisted value is lost.
- The floating bar's window remains the same window with the same close
  handler; "hide" continues to mean hide, never destroy.

## Grilled Decisions (2026-08-14)

- **The menu bar is not a mode.** The previous toggle made the two surfaces
  mutually exclusive; they are now independent. This is the core change and
  the reason the setting is removed rather than defaulted.
- **The bar still shows at launch.** An always-on capture tool that starts
  with no visible window gives a tester no confirmation it is running. The
  menu bar becomes the permanent home for controls without making the first
  launch silent.
- **"Show Action Bar" replaces "Show Floating Bar" and "Show/Hide Encore".**
  One item, one direction: bring the bar back. Hiding is done by closing the
  bar, which is where a user already reaches for it.
- **Double-click is a shortcut, not the only path.** The menu item is the
  guaranteed way to restore the bar; the double-click gesture is an
  accelerator on top of it. See the open question below on click handling.
- **"Gallery" is the Library.** The repository calls this surface the
  Library and the menu says "Open Library"; no second name is introduced.

## Expected Validation

- Rust tests for the menu model: every action present regardless of capture
  state, and Pause/Resume swapping with the live state.
- A Rust test that a settings document containing the removed field still
  loads with its remaining values intact.
- Manual macOS smokes: fresh launch shows the bar; close the bar and drive
  Save Replay, Pause/Resume, Open Library, and Settings entirely from the
  menu bar; "Show Action Bar" restores it; double-click restores it; Quit
  ends the process.

## Resolved: click handling (2026-08-14)

The open question is answered by the dependency, not by preference.
`TrayIconEvent::DoubleClick` exists in Tauri's enum but **the macOS backend
never emits it** — `tray-icon`'s macOS implementation sends only `Click`,
`Enter`, `Leave`, and `Move`; double-click is Windows-only. It does send the
left `Click`/`Down` before opening the attached menu, so the gesture is
recovered by timing two presses in Encore itself (500ms, matching the macOS
double-click interval).

The menu therefore stays on left-click, where it is discoverable, and
double-click is layered on top. Because the first click opens the menu, the
second press may be consumed by the open menu before it reaches Encore —
this is the one behavior that needs a real macOS run to confirm. The menu's
"Show Action Bar" item is the guaranteed path regardless, and if the
gesture proves unreachable the fallback stands: move the menu to
right-click only, freeing left-click entirely.

## Risks and Open Questions
- A tester who closes the bar and does not know about the menu bar icon
  could believe Encore stopped. The menu bar icon and the confirmation
  chime are the mitigations; a first-close hint is possible future work.
