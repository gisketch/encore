---
name: sonata-work
description: Route and complete any Sonata work request through the smallest sufficient workflow. Use for new features, bugs, refactors, chores, experiments, or ideas when Sonata should decide dynamically whether to act immediately or use setup, fix, grill, spec, tickets, implement, and review.
---

# Sonata Work

Drive the user's request to a completed, validated result without making them choose Sonata's internal workflow.

1. Preserve the original request across every phase. Inspect readiness, project context, relevant code and docs, active specs or plans, and working-tree state before asking anything.
2. If readiness is missing or pending, use `$sonata-retrofit` when an established codebase exists; otherwise use `$sonata-setup`. Resume the original request after readiness.
3. Choose the minimum path justified by scope and uncertainty:
   - Bug, regression, test or build failure, performance problem, integration failure, or unexpected behavior: use `$sonata-fix` before editing.
   - Trivial, obvious, low-risk non-bug work: change it immediately and run one cheap check.
   - Clear focused behavior: use `$sonata-implement` directly.
   - Important unresolved product or design decisions: use `$sonata-grill`, then continue after approval.
   - Durable behavior with settled decisions: use `$sonata-spec` when a written contract reduces implementation ambiguity.
   - Broad, multi-context, or delegable work: use `$sonata-tickets` after an approved spec.
   - Existing changes awaiting confidence: use `$sonata-review`.
4. After each phase, reassess instead of forcing the remaining stages. Skip specs, tickets, and separate review when they add no confidence.
5. Implement the smallest complete slice, validate dynamically from its actual risk, and update durable docs when behavior or decisions changed.
6. Use review for broad, risky, or explicitly requested work. Resolve findings, then offer a commit only when requested or when a validated first shell has no commits.

Ask only decisions that materially change behavior or scope. Never scaffold, publish, commit, or mutate external systems without confirmation.
