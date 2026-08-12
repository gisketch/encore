# Runnable Tauri Shell

## Goal

Create the smallest macOS-runnable Encore shell that proves the selected
Tauri, Rust, Svelte, TypeScript, Vite, npm, and Motion stack.

## Acceptance Criteria

- `npm run tauri dev` opens an Encore desktop window on macOS.
- The window presents recording status, the 10-minute retention setting, a
  full-screen capture target, and a disabled save-replay action clearly marked
  as not implemented.
- One restrained recorder-timeline animation uses Motion and respects reduced
  motion.
- Frontend checks/build and Rust formatting, linting, and tests pass.
- No capture, permissions, persistence, FFmpeg, or global-hotkey behavior is
  implied to be working in this slice.

## Context Links

- `docs/project-brief.md`
- `docs/architecture/index.md`
- `docs/quality.md`

## Steps

- [x] Add the official Tauri v2 backend shape and a minimal Svelte/Vite frontend.
- [x] Replace template content with the Encore instrument panel.
- [x] Add Motion and reduced-motion behavior.
- [x] Run frontend, Rust, harness, interaction, and native launch checks.
- [x] Update architecture and quality docs with verified commands.

## Validation

- `npm run check`: passed with zero errors and warnings.
- `npm run build`: passed with a production Vite bundle.
- `cargo fmt --check`: passed from `src-tauri`.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo test`: passed across three empty harnesses.
- Browser interaction smoke: target and retention state updated; console clean.
- `npm run tauri dev`: Vite returned HTTP 200 and `target/debug/encore` ran.
- `./scripts/check-sonata.sh --ready`: final result recorded at handoff.

## Decision Log

- Use plain Svelte 5 with Vite instead of SvelteKit; this shell has no routing.
- Keep this slice honest: visual preview state only, no simulated recording.
- Do not add FFmpeg or capture dependencies until their behavior is implemented.
- Generate desktop icons from `assets/encore-icon.svg`; discard mobile outputs.

## Progress Log

- 2026-08-12: Sonata readiness and local prerequisites verified.
- 2026-08-12: Shell implemented and visually checked at 1181×779.
- 2026-08-12: Native launch and all selected Fast-lane checks passed.
