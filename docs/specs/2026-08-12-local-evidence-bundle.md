# Local Evidence Bundle

> Status: **APPROVED** — internally grilled 2026-08-12.

## Problem and Outcome

`Replay ready` preserves rolling segments but gives the tester no durable file
to inspect or share. Pressing Replay must create one playable, local evidence
bundle and make its location obvious without interrupting capture.

## In Scope

- Automatically packaging the leased replay after either trigger entry point.
- Concatenating compatible fragmented-MP4 segments without re-encoding video.
- Transactional, collision-safe output under `Movies/Encore`.
- An adjacent versioned `metadata.json` containing available environment,
  capture, retention, and evidence facts.
- Typed preparing, saved, and recoverable failure states.
- A native `Open in Finder` action that reveals the saved MP4.
- A bundled FFmpeg sidecar for packaged applications and a deterministic local
  development setup through npm.

## Out of Scope

- Online upload, accounts, Jira, Linear, S3, telemetry, and audio.
- A destination picker or settings screen.
- Re-encoding incompatible segments into one output.
- General application-log collection, redaction, or compression.
- A durable export queue or recovery of an in-progress export after Encore
  itself exits.

## Acceptance Criteria

- `Cmd+Option+R` and the rail Replay button use the same native path: atomically
  lease the completed rolling window, keep capture running, and start packaging
  off the shortcut callback and UI command thread.
- A successful request creates
  `~/Movies/Encore/Encore Replay YYYY-MM-DD HH.mm.ss[ suffix]/replay.mp4` and an
  adjacent `metadata.json`; a collision adds a numeric suffix and never
  overwrites an existing bundle.
- The MP4 is nonempty, normally playable, contains H.264 video, and spans the
  compatible leased segments in evidence order without video re-encoding.
- The bundle is published transactionally: work happens in a hidden partial
  directory and the completed directory appears only after MP4 and metadata
  are both durable. Failed work is cleaned up.
- Metadata uses `schemaVersion: 1` and includes replay ID; creation and evidence
  times; segment count and input bytes; app name/version; OS name/version/arch;
  capture source kind and geometry (never its label or native ID); capture,
  encoder, and retention settings; dropped-frame, gap, and retry counters; and
  the output video filename.
- While packaging, the rail says `Saving replay`. On success it says
  `Replay saved` and provides `Open in Finder`. The webview receives only the
  display name and aggregate metadata, never native input or output paths.
- Only one export runs at a time. Triggers during an active export coalesce into
  that export and do not create another bundle.
- On success the lease is released. On failure the lease and replay facts stay
  available for a manual Retry; a later new trigger may replace that failed
  pending replay only after acquiring its own lease.
- Missing FFmpeg, incompatible inputs, sidecar failure, metadata failure,
  destination failure, and Finder failure use stable local error codes. None
  may present a partial bundle as saved or discard retryable evidence.
- Packaged macOS applications include FFmpeg; testers install no separate
  executable. The npm development workflow prepares the target-specific
  sidecar deterministically before Tauri launches or builds.
- Everything remains on the Mac and capture continues while packaging runs.

## Implementation Constraints and Settled Decisions

- Use FFmpeg's concat demuxer with stream copy. A generated manifest contains
  only native leased paths, is deleted with the partial workspace, and is never
  emitted to Svelte or metadata.
- Validate segment codec, dimensions, pixel format, and stream layout before
  concat. A source or geometry discontinuity fails with
  `export_incompatible_segments`; silent re-encoding is forbidden.
- The default destination is fixed to `Movies/Encore` for this compact-rail MVP.
  A changeable destination remains part of the later lifecycle/settings slice.
- Output discovery is mediated by an opaque saved-replay ID held by the native
  service. `Open in Finder` accepts that ID, not a caller-provided filesystem
  path, and invokes macOS `open -R`.
- Current metadata includes the diagnostic values already owned by Encore.
  There is no safe, bounded structured application log yet, so this slice does
  not create a fake or unbounded log attachment.
- FFmpeg distribution is a separate executable sidecar. The npm-pinned artifact
  and its license/provenance must be documented and copied into Tauri's expected
  target-specific binary name; generated binaries are not source-controlled.
- Export work runs on a worker thread. State transitions and lease ownership
  remain synchronized in Rust; the global-shortcut callback only dispatches.
- Stable codes are `export_busy`, `export_ffmpeg_unavailable`,
  `export_incompatible_segments`, `export_concat_failed`,
  `export_destination_unavailable`, `export_metadata_failed`, and
  `export_reveal_failed`.

## Expected Validation

- Golden-media integration coverage packages generated compatible segments,
  parses `metadata.json`, and verifies the output with `ffprobe`.
- Failure tests cover missing/failed FFmpeg, incompatible segments, naming
  collisions, partial cleanup, and preservation of the pending lease.
- Replay-service tests cover preparing/saved/failed transitions, in-flight
  coalescing, successful lease release, and retry.
- Frontend checks/build cover truthful Saving, Saved, Retry, and Finder actions.
- A live macOS smoke records at least two completed segments, triggers Replay,
  verifies the resulting MP4 with `ffprobe`, and reveals it in Finder.
- Rust formatting, Clippy, all tests, Sonata harness, SCC, and diff gates pass.

## Risks and Open Questions

- FFmpeg redistribution carries license and provenance obligations. Keep the
  selected binary version/checksum/license visible and review them before a
  public signed release.
- Logs remain deferred until Encore has a structured, bounded, redacted local
  log source with a clear retention contract.
- A replay crossing a geometry/source discontinuity cannot become one MP4
  without a later normalization/re-encode policy; the MVP fails honestly and
  preserves the evidence for retry or a newer trigger.

## Internal Grill Record

1. **Does Replay merely reserve evidence or save it?** Save it automatically;
   the user's single action must finish with a durable artifact.
2. **Where does it go?** A visible `Movies/Encore` folder, matching the project
   brief and requiring no destination setup.
3. **One file or a bundle?** One timestamped folder containing `replay.mp4` and
   `metadata.json`, so adjacent evidence cannot be separated accidentally.
4. **Should the rail ask for a filename?** No. Timestamped collision-safe names
   preserve the zero-discipline goal.
5. **What happens to capture during export?** It continues; packaging consumes
   leased completed files on a worker.
6. **What happens on rapid or concurrent triggers?** They coalesce into the one
   active export. An unbounded export queue would violate disk bounds.
7. **When is the source lease released?** Only after the completed bundle is
   published. Failures retain it for Retry.
8. **How are partial results handled?** Build in a hidden partial directory,
   remove it on failure, then atomically rename the complete bundle.
9. **Can an old replay be overwritten?** Never. A numeric suffix resolves the
   rare same-second collision.
10. **How is concat performed?** FFmpeg concat demuxer plus `-c copy`; no video
    encode work is added to the hot path.
11. **What if segments differ?** Fail explicitly and retain them. Quietly
    re-encoding would violate performance and fidelity constraints.
12. **How does a tester find the file?** The success state exposes one native
    `Open in Finder` action that reveals `replay.mp4`.
13. **May Svelte receive a saved path?** No. It receives an opaque ID and safe
    display name; Rust resolves the ID to the owned path.
14. **Which metadata is useful and safe now?** Version, OS, architecture,
    source kind/geometry, settings, counters, and evidence bounds; no source
    label, source ID, or input path.
15. **Are logs included now?** No. No bounded structured log exists yet, and an
    empty or indiscriminate attachment would be misleading or unsafe.
16. **Does a tester install FFmpeg?** No. npm prepares a pinned sidecar for
    development/build and Tauri bundles it for distribution.
17. **Is destination customization included?** Not in the compact MVP rail;
    it belongs in the later lifecycle/settings spec.
18. **What survives an Encore crash during export?** The rolling segments on
    disk survive under existing recovery; a partial bundle is discarded on
    startup or the next export. Durable export-job recovery is deferred.
