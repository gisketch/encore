# Quality

Keep this as the project verification menu. Add commands only after they pass locally.

## Harness Checks

| Check | Command | Run When |
|---|---|---|
| Harness structure and source size | `./scripts/check-sonata.sh` | After harness, docs, or skill changes |
| Optional changed-code gates | `node scripts/check-quality-gates.mjs` | Before handoff when SCC or Skylos is enabled |

SCC 3.7.0 is enabled with the language-specific ceilings measured after the
macOS capture slice. Run `node scripts/check-quality-gates.mjs` before handoff;
the gate rejects complexity increases in changed files and new files above the
stored ceiling. Skylos 4.29.0 remains deferred. Retain the project-owned strict
defaults in `.sonata/skylos.toml` when it is enabled.

## Project Checks

| Check | Command | Status |
|---|---|---|
| Bootstrap/install | `npm install` | verified 2026-08-12 |
| Run application | `npm run tauri dev` | verified 2026-08-12; stop with Ctrl-C |
| Frontend checks | `npm run check` and `npm run build` | verified 2026-08-12 |
| Rust checks | `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` from `src-tauri` | verified 2026-08-12 |
| Native launch | `npm run tauri dev`; the configured macOS rpath supplies Apple's Swift runtime for ScreenCaptureKit | verified on macOS 26.5, 2026-08-12 |
| Hardware video writer | `cargo test writes_hardware_h264_smoke_segment -- --ignored --nocapture` from `src-tauri`, then inspect the printed MP4 with `ffprobe` | verified on macOS 26.5, 2026-08-12 |
| Live video pipeline | Run `npm run tauri dev` for over 10 seconds, then inspect completed MP4s under the application cache `rolling-segments` directory with `ffprobe` | verified: H.264, 1512x982, 10.02s, 2026-08-12 |
| Rolling retention | `cargo test retention -- --nocapture` from `src-tauri` | verified 2026-08-12: recovery, interrupted-file cleanup, boundary pruning, duration changes, lease safety, and rejection cases |
| Replay export lifecycle | `cargo test replay -- --nocapture` from `src-tauri` | verified 2026-08-12: in-flight coalescing, saved/failed typed state, successful lease release, retryable failed lease, and opaque Finder reveal lookup |
| Evidence packager | `cargo test packager -- --nocapture` from `src-tauri` | verified 2026-08-12: schema-v1 metadata, collision-safe publish, manifest cleanup, and injected runner/filesystem failures |
| FFmpeg sidecars | `npm run prepare:ffmpeg-sidecars && npm run check:ffmpeg-sidecars` | verified on Apple Silicon, 2026-08-12; generated binaries are ignored |
| Permission UI smoke | Run `npm run dev`, inspect at desktop and 680px widths, and switch 5/10-minute retention; capture/save remain disabled in browser preview | verified 2026-08-12 |
| Global replay/export smoke | Run `npm run tauri dev`; focus another app; hide Encore from its tray menu; press `Cmd+Option+R`; confirm Encore reappears without keyboard focus, says `Saving replay`, then `Replay saved`; inspect the generated MP4 with `ffprobe` and use `Open in Finder`. | verified on macOS 26.5, 2026-08-12: Finder remained focused, MP4 was playable, and Finder reveal selected it |
| SCC changed-code gate | `node scripts/check-quality-gates.mjs` | verified 2026-08-12 |
| Exercise primary behavior | Capture at least two segments, trigger Replay, verify the local bundle and metadata, then reveal its MP4 in Finder while capture continues. | verified 2026-08-12: live 20.04s export continued capture; rebuilt `.app` exported recovered segments through bundled sidecars |
| Observe failures | Lifecycle transitions and stable-coded failures for permission, capture, retention, and export append as JSON Lines to `~/Library/Logs/com.gisketch.encore/diagnostics.jsonl` (Tauri's `app_log_dir`), rotating to `diagnostics.jsonl.1` at ~2MB; inspect with `tail -f ~/Library/Logs/com.gisketch.encore/diagnostics.jsonl` while reproducing a failure (e.g. deny permission, or close the captured window), or `cat` it afterward | verified 2026-08-13 by deterministic `cargo test` coverage (record shape, rotation, write-failure no-op, and a source-loss-then-retry sequence read back in order); manual macOS deny-permission/close-window smoke still outstanding |
| Reset/cleanup | Startup removes interrupted and zero-byte rolling files while leaving unrecognized files untouched; a user-invoked reset remains planned | partial |

## Risk Lanes

- Fast: docs, copy, styling, scaffolding, one-line config. One cheap check; no test required.
- Behavior: branches, parsing, state transitions, regression fixes. One public-seam test plus relevant build/typecheck.
- Critical: persistence, concurrency, security, permissions, money, external contracts. Focused integration evidence and review.
- Milestone: broad or cross-cutting work. All relevant verified checks.

## Quality Bar

- Acceptance behavior exists before broad implementation.
- Validation is reproducible by another agent.
- Planned commands stay marked planned until verified.
- Source files above 350 lines fail the smell check. Required exceptions live in `.sonata/large-files.txt`, never product code.
- New decisions update durable repo context.
- Repeated failures become docs, checks, fixtures, logs, or clearer boundaries.
