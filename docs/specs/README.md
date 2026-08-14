# Specifications

Canonical product and behavior specifications live here.

Name specs `YYYY-MM-DD-short-slug.md`. Keep them proportional to risk and ambiguity. Tracker items may link here but do not replace these files.

## MVP Spec Map

Items marked `NEED GRILLING` are overview specs, not implementation contracts.
Grill and approve one before splitting it into tickets or implementing it.

| Order | Spec | Outcome | Status |
|---|---|---|---|
| 1 | [macOS capture and permissions](2026-08-12-macos-capture-permissions.md) | Obtain frames from a selected display or window with honest TCC recovery | APPROVED |
| 2 | [Video pipeline and segment contract](2026-08-12-video-pipeline.md) | Produce timestamped hardware-H.264 fragmented MP4 segments | APPROVED |
| 3 | [Rolling buffer and crash recovery](2026-08-12-rolling-buffer.md) | Retain only the selected replay window without losing crash evidence | APPROVED |
| 4 | [Replay trigger and snapshot](2026-08-12-replay-trigger.md) | Freeze the current replay window from a global hotkey without stopping capture | APPROVED |
| 5 | [Local evidence bundle](2026-08-12-local-evidence-bundle.md) | Export a playable MP4 with local metadata | APPROVED |
| 6 | [Always-on lifecycle and control](2026-08-12-always-on-lifecycle.md) | Make the complete pipeline understandable and dependable during daily QA work | APPROVED |

## Paper & Grain UI Migration

Migration to the approved "paper & grain" design direction (Claude Design
project "Encore Mockups"). Suggested order below; the foundation and action
bar are near-implementation-ready, later surfaces are new product scope.

| Order | Spec | Outcome | Status |
|---|---|---|---|
| 1 | [Paper & grain design system](2026-08-13-paper-grain-design-system.md) | Shared token layer, typography, grain, and light/dark/system theming | APPROVED |
| 2 | [Action bar redesign](2026-08-13-action-bar-redesign.md) | Collapsed/expanded floating bar replacing the current rail, all states preserved | APPROVED |
| 3 | [Settings window](2026-08-13-settings-window.md) | Persisted recording, saving, hotkey, and appearance settings | APPROVED |
| 4 | [Replay library](2026-08-13-replay-library.md) | Browse, search, open, and delete saved replays grouped by day | APPROVED |
| 5 | [Replay editor](2026-08-13-replay-editor.md) | Trim and cut a saved replay with lossless export | APPROVED |

The Settings window shares persistence and menu-bar decisions with the
always-on lifecycle spec; grill them together so answers stay consistent.

## Post-MVP Behavior

| Spec | Outcome | Status |
|---|---|---|
| [Post-save replay preview](2026-08-14-post-save-preview.md) | Sound plus a corner picture-in-picture preview with Edit, Share, and Open Folder after a save | APPROVED |
| [Menu bar as the primary control surface](2026-08-14-menu-bar-control-surface.md) | Bar shown at launch and hidden by closing it; the menu bar always carries every action | APPROVED |

System audio remains a future milestone. It should receive its own capture and
A/V synchronization spec after the video-only MVP is proven.
