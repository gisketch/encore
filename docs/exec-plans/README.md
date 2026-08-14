# Execution Plans

Use checked-in plans for multi-step or high-risk work.

## Active Plans

- [Replay trigger and snapshot](active/2026-08-12-replay-trigger.md) — RT-01 and
  RT-02 approved for sequential xHigh implementation.
- [macOS capture and permissions](active/2026-08-12-macos-capture.md) — all four
  tickets implemented; permission-dependent hardware matrix remains to run.
- [Always-on lifecycle](active/2026-08-13-always-on-lifecycle.md) — AL-01
  through AL-04 drafted from the approved lifecycle spec.
- [Paper & grain UI migration](active/2026-08-13-paper-grain-ui.md) — PG-01
  through PG-15 implemented and reviewed (PG-09 start-at-login remains
  deferred); manual macOS smokes of the new windows remain outstanding.
- [Menu bar control surface](active/2026-08-14-menu-bar-control-surface.md) —
  MB-01 through MB-03 drafted from the approved spec; starts with MB-01.
- [Post-save preview](active/2026-08-14-post-save-preview.md) — PP-01 through
  PP-05 implemented and reviewed; the end-to-end macOS smoke and a JS test
  runner for the auto-dismiss timing rule remain outstanding.

Use this name pattern for new plans:

```text
YYYY-MM-DD-short-slug.md
```

## Completed Plans

- [Runnable shell](completed/2026-08-12-runnable-shell.md)
- [macOS video pipeline](completed/2026-08-12-video-pipeline.md) — VP-01 through
  VP-03 implemented; native hardware and live-capture media checks pass.
- [Rolling buffer and crash recovery](completed/2026-08-12-rolling-buffer.md) —
  RB-01 and RB-02 implemented; final Sonata review is clean.
- [Local evidence bundle](completed/2026-08-12-local-evidence-bundle.md) — LEB-01
  and LEB-02 implemented; live dev and packaged-app exports plus Finder reveal
  pass final Sonata review.

## Required Sections

- Goal.
- Acceptance criteria.
- Context links.
- Steps.
- Validation.
- Decision log.
- Progress log.
