---
name: sonata-grill
description: Stress-test a Sonata plan or design through a one-question-at-a-time interview. Use when requirements, boundaries, terminology, tradeoffs, or acceptance behavior need sharpening before implementation.
---

# Sonata Grill

Resolve design branches without overwhelming the user.

If Sonata setup is pending, mention it but continue; grilling remains available for planning and diagnosis.

1. Read relevant repo facts and active specs first. Look up facts; ask only decisions.
2. Ask exactly one question per turn. Give the recommended answer and its main tradeoff.
3. Walk dependencies in order: outcome, users, boundaries, behavior, failure cases, data, operations, validation.
4. Challenge vague terms and conflicting requirements directly.
5. Update the working file under `docs/specs/` when a decision settles.
6. Create or update a glossary only when domain terms need durable definitions.
7. Offer an ADR only when the decision is hard to reverse, surprising, and based on a real tradeoff.
8. Stop when acceptance behavior, scope, and open risks are explicit. Ask for confirmation before implementation.

Do not implement during the grill.
