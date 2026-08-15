---
name: sonata-implement
description: Implement an approved Sonata spec or ticket through the repository harness. Use for feature work, fixes, refactors, or configuration changes that require risk-based validation and durable handoff.
---

# Sonata Implement

Implement one focused slice. Read its spec, architecture, and `docs/quality.md` first.

Choose the smallest credible risk lane from `docs/quality.md`; do not add ceremony beyond it.

## Loop

1. Run `./scripts/check-sonata.sh --ready`. If it fails, stop and direct the user to `$sonata-setup`.
2. Confirm acceptance criteria and smallest complete slice.
3. Search existing patterns and shared code before adding anything.
4. Create or update an execution plan only for multi-step or high-risk work.
5. Implement the smallest durable change.
6. Run one targeted check while working and one final check for the chosen lane. Do not repeat an expensive passing check unless relevant code changed.
7. Update docs when behavior, commands, boundaries, or decisions changed.
8. If the same failure recurs, improve the harness with a doc, check, fixture, log, or clearer boundary.

If this created the first runnable shell and Git has no commits, offer an initial commit after validation. Never commit automatically. Otherwise commit only when the user or repo workflow requests it.
