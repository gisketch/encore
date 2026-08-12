---
name: sonata-tickets
description: Split an approved Sonata spec or plan into small vertical tickets with explicit blockers. Use when work must be sequenced, delegated, published to an issue tracker, or completed across separate agent contexts.
---

# Sonata Tickets

Create demoable tracer bullets, not layer-by-layer chores.

1. Run `./scripts/check-sonata.sh --ready`. If it fails, stop and direct the user to `$sonata-setup`.
2. Read the approved spec and relevant architecture. Do not invent missing product decisions.
3. Draft the fewest vertical slices that each deliver observable behavior and fit one fresh context.
4. Put prefactoring first only when it genuinely unlocks later slices.
5. Give each ticket a title, delivered behavior, acceptance criteria, validation, and `Blocked by` edges.
6. Keep wide mechanical migrations as expand, migrate, contract tickets when independent slices cannot stay green.
7. Add the approved breakdown to the relevant file under `docs/exec-plans/active/`.
8. If `docs/issue-tracker.md` exists, publish coordination copies in blocker order and link each to the canonical spec.
9. Work only tickets whose blockers are complete.

Show the draft before publishing externally. Ask whether granularity and blockers are correct.
