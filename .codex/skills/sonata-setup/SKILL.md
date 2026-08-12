---
name: sonata-setup
description: Configure or repair a new or already-understood Sonata project harness. Use for greenfield setup, issue tracking, project context, verification commands, or optional SCC and Skylos quality gates; use sonata-retrofit for deep adoption of an established codebase.
---

# Sonata Setup

Interview the project until it has enough explicit context to begin its current milestone.

1. Read the manifest and repository map. Inspect source, package/build files, tests, remotes, Git state, and existing docs before asking anything.
2. Ask exactly one unresolved decision per turn. Never ask for facts the repository already answers. Give one recommendation and its main tradeoff.
3. Resolve and record in `docs/project-brief.md`:
   - Product Vision: the long-term destination.
   - Users and operating environment.
   - Current Milestone: the next useful outcome.
   - Non-Goals: what the product fundamentally will not do.
   - Later / Not Now: valid ideas deferred beyond this milestone.
   - Observable acceptance behavior and durable constraints.
4. Detect an existing stack. For greenfield work, ask whether the user has one in mind; otherwise recommend one primary stack and one alternative. Record the choice without installing or scaffolding it.
5. Detect Git with `git rev-parse --show-toplevel`. If no repository exists, recommend `git init`; run it only after confirmation and never create a commit automatically. Declining Git does not block readiness.
6. Resolve issue tracking: Local only (recommended), GitHub Issues, Linear, or Other with custom input. Write `docs/issue-tracker.md`; keep credentials out and do not create external resources without explicit approval.
7. Discover bootstrap, run, primary-behavior, failure-observation, reset, and other project checks. Verify commands when an application exists; mark them Planned for greenfield work. Do not ask for a quality posture—validation is chosen dynamically from each task's scope and risk.
8. Resolve optional quality gates from `.sonata/quality-gates.json`, one decision per turn. Changed-code gates require Git; when Git is unavailable, leave them disabled and explain how to revisit setup later.
   - Ask whether to enable SCC for file-level complexity. Recommend it when source files exist. If accepted, require the pinned version, run `node scripts/check-quality-gates.mjs --recommend-scc`, explain that recommendations use each language's observed 75th percentile, and confirm the language-specific ceilings before writing them.
   - Ask whether to enable Skylos for changed-code security, secrets, and quality. Recommend it for supported source languages. If accepted, require the pinned version and keep the project-owned defaults in `.sonata/skylos.toml` unless the user chooses different thresholds.
   - Never install a tool without confirmation. For SCC, offer the pinned GitHub release or platform package manager. For Skylos, offer `uv tool install skylos==<version>` or `python -m pip install skylos==<version>`.
   - When a gate is enabled, add `node scripts/check-quality-gates.mjs` to `docs/quality.md` and verify it. A missing, mismatched, or failing enabled tool blocks readiness; a disabled tool is never invoked.
9. If GitHub Actions is detected and at least one gate is enabled, ask whether to add `.github/workflows/sonata-quality.yml`. Use checkout with `fetch-depth: 0`, Node 20, Python 3.11, `node scripts/install-quality-tools.mjs`, then `node scripts/check-quality-gates.mjs`. The installer reads exact versions from `.sonata/quality-gates.json`; never duplicate policy in workflow YAML. Do not create CI without confirmation.
10. Update architecture with known facts only. Preserve useful existing docs and keep `AGENTS.md` a map.
11. Present a final readiness summary. Resolve every missing checklist item before finishing.
12. Run `./scripts/check-sonata.sh` and the narrowest available project check. Then set manifest `setup` to `{ "status": "ready", "version": 1, "completedAt": "<ISO timestamp>" }`.
13. After readiness, detect whether a runnable project shell exists. If absent and a stack is selected, offer a direct handoff to `$sonata-implement` to create the smallest runnable shell. If one exists, offer `$sonata-work` to begin the Current Milestone. Do not scaffold inside setup.

Pending projects remain pending if interrupted. When rerunning setup on a ready project, keep the old ready state until the revised interview completes successfully.
