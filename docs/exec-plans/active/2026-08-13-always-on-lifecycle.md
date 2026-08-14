# Always-on Lifecycle and Control Plan

## Goal

Make Encore dependable across a full tester workday: settings survive
relaunch, pause and system sleep never destroy or misrepresent evidence, source
loss is visible with a recovery path, and a local log can explain any failure.

## Acceptance Criteria

- The behavior in the [approved always-on lifecycle specification](../../specs/2026-08-12-always-on-lifecycle.md)
  is observable at the settings, capture-lifecycle, tray/rail, and log seams.
- No lifecycle path silently discards retained evidence or claims capture that
  the native state machine has not confirmed.
- Every ticket's focused validation and the milestone lane pass.

## Context

- [Always-on lifecycle specification](../../specs/2026-08-12-always-on-lifecycle.md)
- [Replay-trigger specification](../../specs/2026-08-12-replay-trigger.md)
- [Rolling-buffer specification](../../specs/2026-08-12-rolling-buffer.md)
- [Architecture](../../architecture/index.md)
- [Quality menu](../../quality.md)

## Tickets

### AL-01 — Persist tester settings across relaunch

**Delivered behavior:** The capture target choice and retention minutes
survive quit and relaunch through one versioned, atomically written settings
document; corrupt or missing settings never block startup.

**Acceptance criteria:**

- Changing retention or the capture source writes one versioned JSON document
  atomically (write-temp-then-rename) in the app config directory.
- Relaunch restores the persisted retention window and resolves the persisted
  target; a window target persists as best-effort identity (app bundle plus
  title).
- An unresolvable window target falls back to the default display and surfaces
  a visible source notice; the UI never claims the unresolved target.
- A missing or corrupt file yields defaults (full screen, 10 minutes) and
  startup proceeds normally.

**Validation:** Deterministic Rust tests for round-trip, corrupt-file fallback,
atomic write, and unresolvable-target fallback; frontend check/build for the
source notice; a manual relaunch smoke. Behavior lane plus persistence-focused
evidence.

**Blocked by:** None.

### AL-02 — Pause and resume without losing evidence

**Delivered behavior:** An explicit pause from the rail or tray stops new
segment production and freezes wall-clock pruning so paused evidence stays
replay-triggerable; resume restarts capture into the same target and resumes
normal pruning.

**Acceptance criteria:**

- Pause transitions capture to the existing `paused` state, distinct from
  failure states, and is visible in the rail and tray.
- While paused, the retention pruner idles, retained segments remain
  triggerable, and disk use stays bounded because nothing new is written.
- Resume restarts capture into the same target through one resume path and
  restores normal pruning.
- Rail copy states the retained-window age honestly while paused.

**Validation:** Deterministic Rust tests for pause freezing pruning, trigger
eligibility while paused, and resume; frontend check/build; manual
pause/trigger/resume smoke. Behavior lane.

**Blocked by:** None.

### AL-03 — Survive sleep, wake, and source loss

**Delivered behavior:** Sleep and stream errors transition to a recovering
state and auto-resume with bounded retries through the AL-02 resume path;
losing the captured source lands in a visible `source_unavailable` state with
retry, re-pick, and full-screen fallback actions.

**Acceptance criteria:**

- Sleep or a stream error transitions to `recovering`; wake auto-restarts the
  same target with one bounded retry policy shared by all recovery paths.
- Exhausted retries land in `failed` with a one-click retry action.
- Recovery keys off stream errors and clock discontinuities, not only power
  events.
- Source loss shows `source_unavailable` with retry, source re-pick, and
  full-screen fallback; no silent target switching.
- Lock/unlock receives no special handling and makes no locked-screen claims.

**Validation:** Deterministic Rust tests for retry exhaustion and source-loss
transitions; frontend check/build for recovering/unavailable presentation;
manual macOS sleep/wake and window-close smokes. Critical lane.

**Blocked by:** AL-02.

### AL-04 — Explain failures from a local log

**Delivered behavior:** Every lifecycle transition and stable-coded failure for
permission, capture, retention, and export appends to a size-capped local JSON
Lines log that can answer "why did today's capture stop?".

**Acceptance criteria:**

- Transitions and failures append one structured record with timestamp, typed
  state, and stable code to the app log directory.
- The log is local-only, JSON Lines, size-capped with rotation; local paths may
  appear in the log but never in webview state.
- A manual failure (deny permission or close the captured window) is
  reconstructable from the log alone.
- `docs/quality.md` marks the observe-failures check verified with the log
  location and inspection command.

**Validation:** Rust tests for record shape and rotation; a manual
inject-failure-then-read-log smoke. Behavior lane.

**Blocked by:** AL-03.

## Validation Lane

Milestone: this crosses persistence, capture lifecycle concurrency, OS power
events, and user-visible truthfulness. Run focused ticket checks while working
and every relevant verified project check before review.

## Decision Log

- 2026-08-13: Internally grilled and approved — close hides, menu-bar accessory
  only, pause freezes pruning, one shared bounded recovery path, persisted
  settings limited to target and retention, launch-at-login and notifications
  deferred.
- 2026-08-13: Local repository documents are authoritative; no external tracker
  publication is configured.

## Progress Log

- 2026-08-13: Grill completed, canonical spec approved, tickets drafted.
- 2026-08-13: AL-01 landed. Added `capture/settings` (`SettingsDocument`,
  `PersistedTarget`, `SettingsStore`) with a write-temp-then-rename atomic
  save and a default-on-missing-or-corrupt load; unknown JSON fields are
  ignored and an out-of-range retention value is normalized rather than
  failing the document. `CaptureService::new` now takes an injected settings
  path (resolved via `app.path().app_config_dir()` in `lib.rs::run`), loads
  the persisted retention minutes into `RollingStore::open`, and resolves the
  persisted target at startup (`capture/service/persistence.rs`): a window
  target resolves by best-effort identity (`bundle_id` + `title`, both new
  `CaptureSource` fields) against `list_sources()`, falling back to the
  default display and setting the new `CaptureSnapshot.source_notice` field
  (`persisted_window_unavailable`) when it can't be found. `switch_by_id` and
  `set_retention_minutes` persist on success. `ReplayStatus.svelte` surfaces
  `sourceNotice` in its detail line. Split `capture/model.rs` and
  `capture/service.rs` into directories (`model/tests.rs`,
  `service/persistence.rs`, `service/tests/persistence.rs`) to stay under the
  350-line smell threshold.
  Tests added: `settings_round_trip_through_atomic_write`,
  `atomic_write_leaves_no_temporary_file_behind`,
  `missing_file_yields_defaults_and_does_not_error`,
  `corrupt_file_yields_defaults_without_blocking_startup`,
  `out_of_range_retention_falls_back_to_the_default`,
  `unknown_fields_are_ignored_rather_than_fatal`,
  `window_target_resolves_by_bundle_and_title`,
  `unresolvable_window_target_falls_back_to_default_display`,
  `display_target_never_resolves_to_a_specific_source` (all in
  `capture/settings/tests.rs`), plus
  `unresolvable_persisted_window_falls_back_to_default_display_with_notice`,
  `resolvable_persisted_window_restores_without_a_notice`, and
  `switching_source_persists_it_for_the_next_launch` in
  `capture/service/tests/persistence.rs`. Found and fixed a real atomic-write
  bug during testing: the temp filename was derived only from the process
  id, so two `SettingsStore`s writing to different target paths in the same
  directory raced on the same temp file; the fix keys the temp filename off
  the target file name too.
  Checks passed: `cargo fmt --check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test` (67 passed, 3 ignored,
  re-run 5x with no flakes), `npm run check`, `npm run build`,
  `./scripts/check-sonata.sh`, `node scripts/check-quality-gates.mjs`.
- 2026-08-13: AL-02 landed. Added `CaptureService::pause`/`resume`
  (`capture/service/pause.rs`): `pause` requires the service to be in
  `Capturing`, transitions to the existing `Paused` state, tears down the
  running stream, sets `pipeline` back to `Idle`, and tells the encoder to
  finalize (`EncoderCommand::Stop`) — distinct from the monitor's automatic
  blank/suspended pause, which leaves the stream running, and distinct from
  `stop`, which clears the source. `resume` requires `Paused`, then calls the
  same `switch_source_locked` restart path `switch_by_id`/`retry` already
  use, so AL-03 wake recovery can reuse it verbatim. Retention pruning
  needed no new code: `RollingStore` only prunes relative to the
  most-recently-admitted segment, so withholding new segments while paused
  already freezes the retained window, and replay eligibility
  (`rolling_store().lease()`) was already independent of capture state.
  Exposed `pause_capture`/`resume_capture` Tauri commands
  (`src-tauri/src/lib.rs`) invoked by both the rail (`RailActions.svelte`,
  `CaptureShell.svelte`) and the tray. `desktop.rs` gained `Pause
  Capture`/`Resume Capture` tray menu items whose enabled state is kept in
  sync with live capture state via `wire_capture_menu`, which listens for
  `capture-state-changed` and toggles each item; both items call the same
  service methods the rail uses. `ReplayStatus.svelte` now gives the paused
  state its own honest copy — "Not recording — keeping last N min" — instead
  of falling through to the default detail line, which could otherwise
  read like a live source label. Split the capture-startup concurrency probe
  test out of `capture/service/tests/mod.rs` into
  `capture/service/tests/concurrency.rs` to keep `mod.rs` under the 350-line
  smell threshold after adding the shared `service_with_rolling` test
  helper.
  Tests added (all in `capture/service/tests/pause.rs`):
  `pause_requires_active_capture`,
  `pause_stops_the_stream_and_tells_the_encoder_to_finalize`,
  `resume_requires_paused_capture`,
  `resume_restarts_capture_into_the_same_target`,
  `pausing_freezes_retention_and_paused_evidence_stays_replay_triggerable`,
  `resume_admits_new_segments_and_restores_normal_pruning`.
  Checks passed: `cargo fmt --check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test` (70 passed, 3 ignored),
  `npm run check`, `npm run build`. `./scripts/check-sonata.sh` and
  `node scripts/check-quality-gates.mjs` both fail, but only on
  `.claude/worktrees/pg-ui/scripts/quality-gates.mjs` — an unrelated,
  untracked worktree left on disk by a concurrent session; the same failure
  reproduces on the pre-AL-02 baseline with this change stashed out, and the
  file-size portion of `check-sonata.sh` (`file size ok`) passed cleanly.
- 2026-08-13: AL-03 landed. Found that the shared bounded-retry recovery
  mechanism (`capture/service/recovery.rs`, `CaptureState::Recovering`,
  `RETRY_DELAYS`, `switch_source_locked` restart, `retry_exhausted` →
  `Failed`) and the `SourceUnavailable` transition
  (`capture/service/monitor.rs::mark_unavailable`) already existed in the
  working tree from earlier scaffolding, so the remaining gap was: (1) a
  second recovery trigger besides stream errors, and (2) the three visible
  source-loss actions in the rail. Generalized `recovery::recover_transient`
  into a shared `recover(service, source_id, reason)` used by both
  `recover_transient` (`transient_stream_failure`, unchanged behavior) and a
  new `recover_from_clock_discontinuity` (`clock_discontinuity`) — one
  bounded retry policy, still restarting through `switch_source_locked`, the
  same mechanism `resume` uses. Added `RuntimeSignal::ClockDiscontinuity`
  (`capture/mod.rs`) and had the encoder worker
  (`encoder/worker.rs::run`) send it whenever the timeline reports a
  `SegmentBoundary::ClockDiscontinuity` for the active source, instead of
  only cutting a new segment silently — this is how a real sleep/wake gap
  surfaces when ScreenCaptureKit doesn't report a hard stream error, so
  recovery no longer depends solely on power notifications. Wired the new
  signal in `capture/service/monitor.rs`. On the frontend, added a
  `Full screen` fallback action to `RailActions.svelte` for
  `source_unavailable` (wired to the existing `start_capture` command via a
  new `fallbackToFullScreen` handler in `CaptureShell.svelte`), alongside
  the existing `Retry` button and the always-present source-select control
  (re-pick), so all three source-loss actions the spec calls for are
  present; no new dialogs were added. Lock/unlock and lower-level power
  notifications remain untouched — recovery keys off stream errors and
  clock discontinuities as the spec directs.
  Tests added (all in `capture/service/tests/recovery.rs`):
  `stream_error_transitions_capture_to_recovering_before_retrying`,
  `clock_discontinuity_recovers_through_the_same_bounded_path_into_the_same_target`,
  `bounded_retry_exhaustion_lands_in_failed_with_a_stable_code`,
  `source_loss_marks_unavailable_with_a_stable_code_and_keeps_the_original_target`,
  `source_unavailable_state_supports_retry_repick_and_full_screen_fallback`.
  Checks passed: `cargo fmt --check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test` (75 passed, 3 ignored,
  re-ran the new recovery tests to confirm no timing flakes), `npm run
  check`, `npm run build`, `./scripts/check-sonata.sh`, `node
  scripts/check-quality-gates.mjs` (both clean this run — no worktree
  interference observed).
- 2026-08-13: AL-04 landed. Added a new top-level `diagnostics` module
  (`src-tauri/src/diagnostics/{mod.rs,writer.rs}`) providing `DiagnosticLog`:
  a small `Clone` handle wrapping a `Mutex<DiagnosticWriter>` that appends
  one JSON Lines record (`timestampUnixMs`, `domain`, `state`, `code`) per
  `record()` call under a short lock, then flushes with a direct
  `File::write_all` — no background thread, no per-frame cost. Rotation is
  cap-based: once an append would exceed `max_bytes` (2 MB in production,
  injectable via `with_cap` for tests), the active file renames to
  `<path>.1` (overwriting any previous rotation) before a fresh file opens,
  so at most one active file plus one predecessor exist. Any failure to open
  or write the file degrades `DiagnosticWriter.file` to `None` and every
  subsequent `append` becomes a silent no-op — capture, retention, and
  export never see or propagate a logging error. Domains are exactly the
  four boundaries the spec's log question depends on:
  `permission | capture | retention | export`; `label()` renders the
  existing typed enums (`PermissionState`, `CaptureState`, `RetentionState`)
  through their own `#[serde(rename_all = "snake_case")]` so the log speaks
  the same vocabulary as the webview snapshot rather than a parallel string
  mapping. `lib.rs::run` resolves the path via
  `app.path().app_log_dir()?.join("diagnostics.jsonl")` and hands one
  `DiagnosticLog` to both `CaptureService::new` and `ReplayService::new`.
  `CaptureService` calls it from the same seams AL-01..03 already touch:
  `request_permission` (permission changes), the new
  `capture/service/switch.rs` (split out of `service.rs`, which was sitting
  exactly at the 350-line smell threshold, to hold `switch_source[_locked]`
  and `switch_failed` — logs the resulting `capturing`/`paused` state on
  success and the fallback state with its stable code on failure),
  `pause.rs::pause`, `control.rs::stop`, `monitor.rs`'s
  `mark_healthy`/`mark_paused`/`mark_unavailable` (the automatic
  blank/suspended and source-loss transitions) and `record_segment`
  (retention admission failures), and `recovery.rs` (one log line per
  bounded-retry attempt with its reason code, the `retry_exhausted`
  landing, and the `capture_permission_revoked` branch logging both the
  permission and capture domains) — a new `capture/service/diagnostics.rs`
  holds the three narrow `log_permission`/`log_capture`/`log_retention`
  helpers so no logger is threaded through call signatures; every method
  already has `self`. `ReplayService` gained a `diagnostics: DiagnosticLog`
  field and a `fail()` wrapper around the existing `record_failure` state
  helper so every `Err` path also logs `export`/`failed` with its code, plus
  a `saved` log line in `run_export`'s success arm. Updated
  `docs/quality.md`'s "Observe failures" row with the log location
  (`~/Library/Logs/com.gisketch.encore/diagnostics.jsonl`) and a `tail -f`
  inspection command.
  Tests added: `a_recorded_entry_carries_timestamp_domain_state_and_code`,
  `a_record_without_a_code_serializes_a_null_code`,
  `exceeding_the_cap_rotates_the_active_file_to_a_single_predecessor`,
  `a_write_failure_degrades_to_a_silent_no_op`,
  `a_disabled_log_never_touches_disk`,
  `label_renders_the_bare_snake_case_state` (all in
  `diagnostics/tests.rs`), plus
  `a_source_loss_and_manual_retry_sequence_appears_in_the_log_in_order`
  (`capture/service/tests/diagnostics.rs`) — switches into `window:2`,
  marks it unavailable, retries, then reads the on-disk log back and
  asserts `["capturing", "source_unavailable", "capturing"]` in
  non-decreasing timestamp order with the `source_unavailable` code only on
  the middle record, the closest deterministic proxy for "close the
  captured window and read the log."
  Checks passed: `cargo fmt --check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test` (82 passed, 3 ignored),
  `npm run check`, `npm run build`, `./scripts/check-sonata.sh` (file size
  ok — every touched file stayed at or under 350 lines by splitting
  `switch.rs` out of `service.rs`), `node scripts/check-quality-gates.mjs`.
  Not run: a manual macOS deny-permission/close-window smoke against a
  built app (left outstanding in the quality-menu row above); everything
  else in the ticket's validation list is deterministic and automated.
- 2026-08-13: Post-implementation review found and fixed: (P1) an explicit
  user pause could be silently overridden — a queued `TransientFailure`/
  `ClockDiscontinuity` signal restarted capture and a stale `Healthy` signal
  flipped the state to `Capturing` with no stream; a `user_paused` guard on
  the service (set by `pause`, cleared on the next successful capture start)
  now makes recovery and healthy-signal handling bail while it holds.
  (P2) the `persisted_window_unavailable` notice was never cleared —
  `switch_source_locked` now clears `source_notice` on success. (P2) a failed
  diagnostic-log rotation reset the tracked size to zero and defeated the
  cap — rotation now truncates when the predecessor rename fails and keeps
  the real file size. (P3) paused rail copy now reads "keeping the N min
  before pause". Regression tests:
  `a_queued_stream_error_cannot_override_an_explicit_pause`,
  `a_stale_healthy_signal_cannot_unpause_without_a_stream`,
  `resume_clears_the_pause_guard_so_recovery_works_again`,
  `repicking_a_source_clears_the_fallback_notice`,
  `a_failed_rotation_drops_old_content_instead_of_growing_without_bound`.
  87 Rust tests, fmt, Clippy, frontend check/build, Sonata, and SCC all pass.
  Outstanding: manual macOS smokes (relaunch persistence, real sleep/wake,
  deny-permission log readback) and a documented decision on whether the
  full-screen fallback action should persist the target.
