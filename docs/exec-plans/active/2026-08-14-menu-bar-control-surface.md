# Menu Bar Control Surface Plan

Status: **READY** — spec approved (self-grilled 2026-08-14), tickets below.

## Goal

Make the menu bar Encore's permanent, complete control surface, and demote
the floating action bar to an optional window that is shown at launch,
hidden by closing it, and restored from the menu bar.

## Acceptance Criteria

- Behavior in the [approved spec](../../specs/2026-08-14-menu-bar-control-surface.md)
  is observable with the floating bar both shown and hidden.
- No capture, save, library, editor, or preview behavior changes.
- Every ticket's focused validation and the relevant quality lane pass.

## Context

- [Menu bar control surface spec](../../specs/2026-08-14-menu-bar-control-surface.md)
- [Always-on lifecycle spec](../../specs/2026-08-12-always-on-lifecycle.md) —
  close-hides and menu-bar-accessory were already its decisions.
- [Paper & grain plan](2026-08-13-paper-grain-ui.md) — PG-08 shipped the
  optional mode this supersedes; its tray menu model and shared action
  dispatch are what these tickets build on.
- [Architecture](../../architecture/index.md) · [Quality](../../quality.md)

## Tickets

### MB-01 — The menu bar always carries every action

**Delivered behavior:** The tray menu offers Save Replay, Pause/Resume,
Open Library, Settings…, Show Action Bar, and Quit at all times, whether the
floating bar is visible or hidden. The menu model loses its mode parameter;
"Show Floating Bar" becomes "Show Action Bar" and replaces the old
Show/Hide Encore pair.

**Acceptance criteria:**

- The same complete menu appears regardless of the floating bar's
  visibility and of the (still-persisted, now ignored) menu-bar setting.
- Pause/Resume reflects live capture state whenever the menu is opened.
- Every item drives the same path as its floating-bar or hotkey
  counterpart; none reimplements an action.
- "Show Action Bar" restores and focuses the bar from any state.

**Validation:** Rust tests over the pure menu model (completeness for each
capture state, Pause/Resume swap); manual menu exercise with the bar shown
and hidden. Behavior lane.

**Blocked by:** none.

### MB-02 — Retire the "Show in menu bar" setting

**Delivered behavior:** The setting, its Settings → General toggle, its
command, and its persisted field are gone. Launch always shows the floating
action bar; closing the bar hides it; the menu bar is always present.

**Acceptance criteria:**

- Settings → General no longer offers "Show in menu bar".
- A fresh launch shows the floating bar and the menu bar icon.
- A settings document written by a previous version — including one where
  the removed field was true — loads without error, keeps every other
  value, and no longer hides the bar at launch.
- Closing the bar hides it with capture and the hotkey unaffected.

**Validation:** Rust tests for loading a document containing the removed
field and for the surviving values; frontend check/build; manual
relaunch-then-close smoke. Behavior lane (touches persistence).

**Blocked by:** MB-01.

### MB-03 — Double-click the menu bar icon to show the action bar

**Delivered behavior:** Double-clicking the menu bar icon shows and focuses
the floating action bar, as an accelerator for the menu's "Show Action Bar"
item.

**Acceptance criteria:**

- Double-clicking the icon restores the bar when it is hidden and focuses
  it when already visible.
- Opening the menu still works by whatever click the resolved handling
  assigns to it, and the menu's own "Show Action Bar" item keeps working as
  the guaranteed path.
- The resolved click mapping is recorded in the spec and the progress log —
  including the fallback if menu-on-left-click and double-click detection
  turn out to be mutually exclusive.

**Validation:** manual macOS smoke of single click, double click, and the
menu item, with the bar both hidden and visible. Behavior lane.

**Blocked by:** MB-01.

## Dependency Order

MB-01 → MB-02, MB-03 (MB-02 and MB-03 are independent of each other)

## Validation Lane

Behavior lane per ticket. One end-to-end macOS smoke after MB-03: launch,
close the bar, drive every action from the menu bar, restore by menu item
and by double-click, then quit.

## Decision Log

- 2026-08-14: Spec self-grilled and approved; the menu bar stops being a
  mode and becomes unconditional, which is why the setting is removed
  rather than defaulted.
- 2026-08-14: The bar still shows at launch — a capture tool that starts
  with no visible window gives a tester no confirmation it is running.
- 2026-08-14: Click handling for the tray icon is an open question resolved
  by manual smoke in MB-03; the menu item is the guaranteed restore path
  regardless of the outcome.

## Progress Log

- 2026-08-14: Plan created.
- 2026-08-14: MB-01 shipped. `menu_actions` lost its mode parameter and
  always returns Save Replay, Pause/Resume, Open Library, Settings…, Show
  Action Bar, Quit; only the pause item varies, and only with capture
  state. Show Action Bar replaced both Show Floating Bar and the old
  Show/Hide pair, and `show_action_bar` is the single restore path. Tests
  assert no capture state can shrink the menu.
- 2026-08-14: MB-02 shipped. The toggle, its command, its service slice,
  and the persisted `menu_bar_mode` field are gone; `hide_window` went with
  them, having lost its only caller. Removal is tolerant, with tests for
  an old document loading intact and for a rewrite dropping the stale key.
- 2026-08-14: MB-03 shipped, with a platform finding: macOS never emits
  `TrayIconEvent::DoubleClick` (Windows-only in `tray-icon` 0.24), so the
  gesture is timed in Encore from the left `Click`/`Down` edge, which
  macOS does deliver before the menu opens. 500ms threshold, five unit
  tests over the pure predicate. **Needs a real macOS run** to confirm the
  second press is not swallowed by the menu that the first press opened;
  the menu item remains the guaranteed restore path either way.
- 2026-08-14: Outstanding for this plan — the end-to-end macOS smoke:
  launch, close the bar, drive every action from the menu bar, restore by
  menu item and by double-click, then quit.
