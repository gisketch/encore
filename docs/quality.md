# Quality

Keep this as the project verification menu. Add commands only after they pass locally.

## Harness Checks

| Check | Command | Run When |
|---|---|---|
| Harness structure and source size | `./scripts/check-sonata.sh` | After harness, docs, or skill changes |
| Optional changed-code gates | `node scripts/check-quality-gates.mjs` | Before handoff when SCC or Skylos is enabled |

SCC 3.7.0 and Skylos 4.29.0 remain disabled until the post-shell quality-gate
follow-up. Product source now exists, so that follow-up can install both pinned
tools. For SCC, run
`node scripts/check-quality-gates.mjs --recommend-scc`, and confirm the observed
language-specific ceilings before enabling it. For Skylos, install the pinned
tool and retain the project-owned strict defaults in `.sonata/skylos.toml` when
enabling it. Add the GitHub Actions quality workflow only after at least one
gate is enabled.

## Project Checks

| Check | Command | Status |
|---|---|---|
| Bootstrap/install | `npm install` | verified 2026-08-12 |
| Run application | `npm run tauri dev` | verified 2026-08-12; stop with Ctrl-C |
| Frontend checks | `npm run check` and `npm run build` | verified 2026-08-12 |
| Rust checks | `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` from `src-tauri` | verified 2026-08-12 |
| Shell interaction smoke | In the launched shell, switch capture target and 5/10-minute retention; save remains disabled | verified 2026-08-12 |
| Exercise primary behavior | Define a local capture-retain-export smoke test at the public application seam | planned |
| Observe failures | Define structured local logs for permissions, capture, retention, and export | planned |
| Reset/cleanup | Define a safe command that clears disposable rolling segments but preserves saved exports | planned |

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
