# Agent Map

Project: encore

Agents read this first. Keep it short; link to durable detail.

## Default Behavior

- Read [docs/index.md](docs/index.md) before broad changes.
- Keep progress terse and acceptance criteria explicit.
- Treat 250–350 lines as a healthy range, not a target. Above 350 is a smell; split at real boundaries.
- Use the narrowest credible validation lane from [docs/quality.md](docs/quality.md).
- When the same failure recurs, improve the harness with a doc, check, fixture, log, or clearer boundary.

## Knowledge Map

- [Project brief](docs/project-brief.md): intent, users, constraints, milestones.
- [Core beliefs](docs/core-beliefs.md): stable engineering principles.
- [Architecture](docs/architecture/index.md): system map and boundaries.
- [Quality](docs/quality.md): verified checks and risk lanes.
- [Specifications](docs/specs/README.md): canonical behavior contracts.
- [Execution plans](docs/exec-plans/README.md): multi-step work.

<!-- sonata:block=workflow:start -->
## Sonata Workflow

- `sonata-work`: route any request through the smallest sufficient workflow and complete it.
- `sonata-fix`: diagnose broken behavior from evidence and fix its root cause.
- `sonata-setup`: configure project context, commands, and optional tracker workflow.
- `sonata-retrofit`: deeply inspect and ground an established codebase before interviewing.
- `sonata-upgrade`: safely apply Sonata updates, then refresh stale docs only.
- `sonata-audit`: scan the full repository and plan security or complexity remediation.
- Setup, spec, tickets, and implementation use `.sonata/manifest.json` readiness; run `$sonata-setup` while pending.
- `sonata-grill`: resolve one design decision at a time before implementation.
- `sonata-spec`: write concise canonical specs under `docs/specs/`.
- `sonata-tickets`: create vertical slices with explicit blockers.
- `sonata-implement`: implement one slice using risk-based validation.
- `sonata-review`: review Standards, Spec, and Behavior separately.
<!-- sonata:block=workflow:end -->

## Work Loop

1. Clarify outcome and acceptance behavior.
2. Read only relevant repo context.
3. Plan at the smallest useful level.
4. Implement inside documented boundaries.
5. Run relevant sensors and self-correct.
6. Update durable context when reality changes.
