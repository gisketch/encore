---
name: sonata-spec
description: Turn an approved discussion, feature idea, or problem into a concise repository-canonical specification. Use when work needs explicit scope, acceptance behavior, decisions, risks, or validation before tickets or implementation.
---

# Sonata Spec

Write the smallest spec that removes implementation ambiguity.

1. Run `./scripts/check-sonata.sh --ready`. If it fails, stop and direct the user to `$sonata-setup`.
2. Read relevant code, docs, ADRs, and active plans. Use repo vocabulary.
3. Synthesize known decisions. Ask only if a missing answer would materially change behavior or scope; use `sonata-grill` when several decisions remain.
4. Write `docs/specs/YYYY-MM-DD-<slug>.md` with:
   - Problem and desired outcome.
   - In scope and out of scope.
   - Acceptance criteria as observable behavior.
   - Implementation constraints and settled decisions.
   - Validation evidence expected at public seams.
   - Risks and open questions.
5. Keep file paths and code snippets out unless they encode a durable contract better than prose.
6. Link the spec from `docs/specs/README.md`.
7. Publish or update a tracker item only when `docs/issue-tracker.md` configures one. Link back to the repo spec; do not duplicate full canonical context.

No forced user-story count. Length follows risk and ambiguity.
