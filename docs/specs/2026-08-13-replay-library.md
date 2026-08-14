# Replay Library

> Status: **APPROVED** (self-grilled 2026-08-13; decisions below).
> New surface (no code exists); layout settled by mockups 1c (light) and
> 2c (dark).
> Depends on: [Paper & grain design system](2026-08-13-paper-grain-design-system.md),
> [Local evidence bundle](2026-08-12-local-evidence-bundle.md) (bundle layout
> and metadata are the library's data source).

## Problem and Outcome

Saved replays currently land in the export folder and are only reachable via
"Reveal in Finder". Testers accumulate replays across days with no in-app way
to browse, reopen, or clean them up.

Outcome: a Library window listing saved replays grouped by day, with
thumbnails, durations, sizes, search, per-replay open/delete, and a bridge to
the Editor.

## In Scope

- Library window (mockups 1c/2c): title bar with Encore mark, replay count
  and total size (`13 replays · 1.2 GB`), search field, `Open Folder ↗`
  button that reveals the export directory in Finder.
- Day grouping: `Today`, `Yesterday`, then dated chips (`Mon, Aug 10`), each
  with per-day count and size; older days collapsed behind a `Show` button.
- Replay cards: 16:9 thumbnail, duration badge, saved time as title, mono
  sub-line `{duration} · {size}`; a `TRIMMED` badge on replays that were
  edited (depends on Editor spec metadata).
- Card interactions: hover reveals `Open in editor` (primary) and a delete
  affordance; opening without the editor available falls back to the system
  player or Finder reveal (grilling decision).
- Delete: removes the evidence bundle (video + metadata) with confirmation;
  moves to Trash rather than hard-deleting (macOS convention) unless
  grilling decides otherwise.
- Data source: the on-disk export folder is the source of truth. The library
  derives its index from bundle metadata; externally deleted or added files
  are reflected (rescan on window focus at minimum).
- Open Library from: action bar button, ⌘⌥L global hotkey (Settings spec),
  and after-save behavior when configured.

## Out of Scope

- Editing itself (Editor spec).
- Tagging, renaming, notes, sharing/upload.
- Watching the folder in real time (focus-rescan is sufficient for MVP).
- Any cloud/index database beyond a lightweight local cache.

## Acceptance Criteria

- All bundles present in the export folder appear, correctly grouped and
  ordered newest-first; counts and sizes are accurate.
- Search filters visibly (match on date/time text at minimum; scope is a
  grilling question).
- Deleting a replay removes it from disk (to Trash) and from the list;
  deletion failures are surfaced.
- Files removed outside Encore disappear after rescan without errors.
- Thumbnails render for every playable bundle; a placeholder appears (never
  a broken image) when thumbnailing fails.
- Both themes match the mockups.

## Implementation Constraints and Settled Decisions

- Local-only: no network; thumbnail generation uses the bundled FFmpeg
  sidecar or AVFoundation, cached beside or under the app's local data dir,
  never inside the user's export folder uninvited (grilling: cache
  location).
- The library never mutates bundle contents; it reads metadata written by
  the evidence-bundle pipeline.
- Must stay responsive with hundreds of replays (lazy thumbnails,
  incremental rendering).

## Expected Validation

- Fixture folder tests for grouping, sizes, and external add/remove.
- Manual smoke: save → appears under Today; delete → gone from Finder too.
- Thumbnail failure path exercised with a corrupt file fixture.

## Grilled Decisions (2026-08-13)

- Thumbnails: extracted with the bundled ffmpeg sidecar (first keyframe
  ≥ 1 s in), stored as JPEGs in the app cache directory keyed by bundle
  path + file size + mtime; regenerated lazily when the key changes, never
  written into the user's export folder. Failures fall back to a styled
  placeholder card.
- Search scope v1: display name and day-group text only (what the user can
  see). Metadata-field search is future work.
- Before the Editor ships, the card's primary action opens the MP4 in the
  system default player; the Editor spec's integration ticket rewires it to
  `Open in editor`.
- Delete moves the whole bundle folder to the macOS Trash (via the `trash`
  crate) after an in-app confirmation; hard delete is never used.
- `TRIMMED` state lives inside the exported bundle's `metadata.json`
  (written by the editor, with provenance); no sidecar files.
- Index: built by scanning the export folder for bundle folders and reading
  each `metadata.json`; rescan runs on window focus and after any operation
  the library itself performs. No database — the folder is authoritative.

## Risks

- Very large libraries could make focus-rescan noticeable; mitigated by
  lazy thumbnails and comparing directory listings before re-reading
  metadata.
- Bundles predating newer metadata fields must render gracefully (missing
  `trimmed` means untrimmed).
