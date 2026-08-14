# Rolling Buffer and Crash Recovery

> Status: **APPROVED** — internally grilled 2026-08-12.

## Problem and Outcome

Continuous capture must not grow without bound, but the on-disk evidence must
survive the failure being investigated. The desired outcome is a crash-safe
rolling store that retains the selected replay window and reports honestly when
it can no longer protect evidence.

## In Scope

- Directory-derived startup recovery of completed rolling segments.
- Atomic admission of completed segments and rejection of partial segments.
- Five- and ten-minute retention policies.
- Safe pruning that cannot delete leased segments being exported.
- Storage statistics and stable local failure codes at the application seam.
- Connecting the existing duration control to authoritative Rust state.

## Out of Scope

- Video capture or encoding changes, final MP4 export, and user deletion of
  saved clips.
- A proactive minimum-free-space threshold or user-configurable byte cap.
- libobs, system audio, online services, and Windows capture.

## Acceptance Criteria

- At startup, Encore admits non-empty, completed rolling MP4s in chronological
  order and removes stale `.partial.mp4` files left by an interrupted writer.
- A completed segment becomes eligible for replay only after the encoder has
  atomically published its final `.mp4` path.
- With no active export lease, pruning retains the selected five- or ten-minute
  window plus at most one approximately ten-second boundary segment.
- Changing from ten to five minutes prunes immediately. Changing from five to
  ten minutes preserves existing evidence and accumulates the longer window
  from that point forward; deleted history is not fabricated.
- A lease returns a stable, ordered snapshot. Pruning skips leased files until
  the lease is released, after which the next prune may remove them.
- Killing Encore during an active write leaves earlier completed segments
  recoverable on the next launch. A zero-byte final file or `.partial.mp4` is
  removed and never counted as retained evidence.
- Screen capture may continue after a retention failure, but encoding halts so
  disk use cannot grow unbounded. The authoritative state and floating control
  report that evidence retention has failed.
- The UI's 5m/10m selection comes from and writes through the Rust service; it
  does not claim a duration change before the service accepts it.

## Implementation Constraints and Settled Decisions

- Keep the focused native ScreenCaptureKit and VideoToolbox pipeline. The
  [libobs assessment](../research/2026-08-12-libobs-migration-assessment.md)
  found that OBS's replay output is RAM-backed and would regress crash survival.
- The rolling directory is authoritative. Do not add a mutable global index for
  the MVP; final segment names plus filesystem metadata provide recovery order.
- Only regular, non-empty `segment-*.mp4` files in the dedicated rolling
  directory are recoverable. Unrecognized files are left untouched.
- Pruning is based on the newest retained segment rather than wall-clock time.
  Pausing capture therefore does not silently age away the last available
  evidence, and disk use remains bounded because no new files arrive.
- At the 3 Mbps target, ten minutes is about 225 MB of video payload. The normal
  envelope is that payload plus container overhead and one boundary segment.
  Active export leases may temporarily exceed it by the leased snapshot size.
- Saved exports live outside the rolling directory and are never disposable.
- Storage failures use stable local codes and contain no captured content,
  source labels, or filenames in diagnostics exposed to the UI.
- Retention failure is fail-closed for encoding: finish or discard the active
  segment, stop creating new files, and require an application restart to retry
  startup recovery. Failure remains sticky for that process so the UI cannot
  return to healthy while encoding is halted. Screen capture may remain alive
  for source recovery/UI.
- No proactive free-space reserve is enforced in this slice. Write or delete
  failures make retention failed; a future product policy may add a reserve.

## Expected Validation

- Filesystem integration tests in unique temporary directories cover recovery,
  partial cleanup, zero-byte rejection, chronological admission, five- and
  ten-minute pruning, duration changes, and lease-protected files.
- Capture-service tests cover retention snapshot updates and rejected duration
  values at the public Rust seam.
- Frontend type checking and production build verify the authoritative duration
  control and failed-retention presentation.
- Rust formatting, Clippy, tests, Sonata harness, and changed-code complexity
  gates pass.

## Risks and Open Questions

- Filesystem timestamps can be altered by external tools. The rolling directory
  is application-owned; external mutation is not supported in the MVP.
- Proving actual process-kill behavior remains a native smoke test after the
  deterministic recovery cases pass.
- Exact export-lease lifetime is finalized by the replay-trigger spec; this
  slice supplies the deletion-safety primitive only.
- Revisit libobs when Windows, audio mixing, or multi-source composition becomes
  an active milestone and only after a measured feasibility spike.

## Internal Grill Record

1. **What result matters next?** Crash-safe bounded evidence, not replacing a
   working capture stack. The native pipeline stays.
2. **Should OBS's replay buffer become the store?** No. It retains encoded
   packets in RAM until Save, so an Encore crash would erase the evidence.
3. **Index file or directory-derived recovery?** Directory-derived. An index
   adds a second atomicity problem and is unnecessary while final segment files
   already have an atomic publication boundary.
4. **What is a valid segment?** A regular, non-empty `segment-*.mp4` final file
   inside the rolling directory. Partial and zero-byte files are disposable.
5. **How exact is the duration boundary?** Keep the requested interval plus at
   most one segment crossing its oldest edge. This preserves continuous
   evidence without pretending ten-second chunks can be cut precisely.
6. **Does a quiet or paused source age evidence away?** No. Pruning advances
   with newly completed evidence, not idle wall-clock time.
7. **What happens when the tester selects five minutes?** The core applies it
   and prunes immediately. Returning to ten minutes grows forward; deleted
   segments do not return.
8. **How does future export avoid a prune race?** It takes an ordered lease;
   leased paths are skipped until release.
9. **What happens when disk cleanup fails?** Continue capture if possible but
   mark retention failed with a stable, privacy-safe code. Never show a healthy
   evidence state when files are no longer bounded or durable.
10. **Do we reserve free disk space now?** No. The MVP relies on the encoded
    bitrate envelope and reports actual write/delete failure. A reserve needs a
    separate product policy.
11. **When should libobs be reconsidered?** When Windows capture, audio mixing,
    or compositing is committed and its reuse can outweigh FFI, packaging, GPL,
    and runtime costs.
