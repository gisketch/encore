# Local Evidence Bundle Execution Plan

> Status: **COMPLETED** — implemented and reviewed 2026-08-12.

## Goal

Turn either Replay trigger into one playable, discoverable local MP4 and
versioned metadata bundle while capture continues and retryable evidence stays
leased through failures.

## Acceptance Criteria

- The approved [local evidence bundle spec](../../specs/2026-08-12-local-evidence-bundle.md)
  is satisfied without online services or video re-encoding.
- Packaged builds carry FFmpeg as a sidecar; npm prepares it for local Tauri
  development and builds.
- The UI truthfully exposes Saving, Saved, Retry, and Open in Finder states.
- Automated golden-media and failure coverage plus a live local `ffprobe` smoke
  prove the output.

## Context Links

- [Project brief](../../project-brief.md)
- [Architecture](../../architecture/index.md)
- [Quality](../../quality.md)
- [Replay trigger spec](../../specs/2026-08-12-replay-trigger.md)
- [Local evidence bundle spec](../../specs/2026-08-12-local-evidence-bundle.md)

## Tickets

### LEB-01 — Package a leased replay transactionally

**Outcome:** The native core can turn one ordered lease into a collision-safe
`Movies/Encore` bundle containing a stream-copied MP4 and schema-v1 metadata.

**Scope:**

- Add the packager boundary, FFmpeg runner abstraction, concat manifest, input
  compatibility validation, metadata schema, output naming, partial cleanup,
  and atomic publish.
- Add a pinned npm-side FFmpeg acquisition/preparation step, target-specific
  Tauri sidecar configuration, and provenance/license documentation.
- Provide the narrow native inputs needed for capture diagnostics without
  exposing labels, IDs, or paths to the webview.

**Acceptance:**

- Compatible golden segments produce a nonempty stream-copy MP4 and correct
  schema-v1 metadata in a complete bundle.
- Failures use stable codes, clean partial output, retain the caller-owned
  lease, and cannot overwrite an existing bundle.
- Unit/integration tests inject the runner and filesystem failures; a real
  sidecar test is explicitly marked and reproducible.

**Validation:** `cargo test packager -- --nocapture`, `cargo fmt --check`,
Clippy, `npm install`, and a target-sidecar existence/version check.

**Blockers:** None.

### LEB-02 — Connect Replay, UI state, Retry, and Finder reveal

**Outcome:** The hotkey and rail action automatically save the replay, report
truthful progress/failure, and reveal a successful MP4 in Finder.

**Scope:**

- Extend ReplayService to own one export lifecycle and coordinate its lease
  with the packager on a worker thread.
- Coalesce triggers while saving, release leases only on success, preserve
  failed evidence for Retry, and emit typed state changes.
- Add retry and opaque-ID Finder reveal commands plus compact rail presentation
  for Saving, Saved, Retry, and Open in Finder.
- Update architecture and quality docs to match observed behavior.

**Acceptance:**

- Manual and global triggers share the same asynchronous export path and never
  block the shortcut callback.
- State transitions, coalescing, lease release/preservation, retry, and opaque
  reveal lookup have deterministic tests.
- A live run creates a playable local MP4, valid metadata, and a Finder-reveal
  target while capture continues.

**Validation:** all Rust/frontend checks, Sonata/SCC/diff gates, live capture,
`ffprobe` output inspection, and manual Finder reveal.

**Blockers:** LEB-01.

## Implementation Order

1. LEB-01 establishes the transaction, toolchain, and failure vocabulary.
2. LEB-02 consumes that boundary and completes the user-facing vertical path.
3. Sol reviews Standards, Spec, and Behavior separately; findings return to the
   owning implementor before final validation.

## Decision Log

- 2026-08-12: Approved automatic one-action save; no second confirmation step.
- 2026-08-12: Fixed MVP destination to `Movies/Encore`; settings remain later.
- 2026-08-12: Deferred logs because no safe bounded structured source exists.
- 2026-08-12: Selected one in-flight export, stream copy, and honest failure for
  incompatible segment layouts.
- 2026-08-12: Selected an npm-prepared, Tauri-bundled FFmpeg sidecar so the end
  user installs nothing and generated binaries stay out of source control.

## Progress Log

- 2026-08-12: Sol internal grill completed (18 decisions).
- 2026-08-12: Canonical spec approved.
- 2026-08-12: LEB-01 completed: native transactional packager, injected
  runner/filesystem coverage, schema-v1 metadata, and npm-prepared target
  sidecars added.
- 2026-08-12: LEB-02 completed: shared asynchronous manual/global export
  lifecycle, in-flight coalescing, retry-preserved leases, bounded opaque
  Finder reveal, typed rail state, and deterministic Rust/frontend validation
  added.
- 2026-08-12: Sol review found and returned three defects: release builds used
  a source-tree sidecar path, recovered segments had no source ID and could not
  export, and crash-left partial workspaces were not cleaned. All three gained
  regression coverage and were fixed before acceptance.
- 2026-08-12: Live capture exported a 20.04-second H.264 MP4 while later
  segments continued. The rebuilt packaged app exported a 240.51-second H.264
  MP4 through its bundled sidecars. Metadata v1 passed privacy inspection,
  `Open in Finder` selected the MP4, and the global shortcut preserved Finder
  focus.
- 2026-08-12: Final milestone gates passed: 52 Rust tests, Clippy, formatting,
  the real sidecar golden-media test, Svelte checks/build, packaged `.app`
  build, Sonata, SCC, and diff checks.
