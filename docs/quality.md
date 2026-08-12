# Quality

Keep this as the project verification menu. Add commands only after they pass locally.

## Harness Checks

| Check | Command | Run When |
|---|---|---|
| Harness structure and source size | `./scripts/check-sonata.sh` | After harness, docs, or skill changes |
| Optional changed-code gates | `node scripts/check-quality-gates.mjs` | Before handoff when SCC or Skylos is enabled |

SCC 3.7.0 is intentionally disabled during greenfield setup because no product
source exists. Revisit it after the runnable Rust/TypeScript shell is created:
install the pinned tool, run
`node scripts/check-quality-gates.mjs --recommend-scc`, confirm the observed
language-specific ceilings, and then enable the gate.

## Project Checks

| Check | Command | Status |
|---|---|---|
| Bootstrap/install | `npm install` | planned until the Tauri shell exists |
| Run application | `npm run tauri dev` | planned until the Tauri shell exists |
| Fast code checks | Define npm checks plus Rust formatting, linting, and tests with the shell | planned |
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
