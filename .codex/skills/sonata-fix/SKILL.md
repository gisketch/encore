---
name: sonata-fix
description: Diagnose and fix bugs, regressions, test failures, performance problems, build failures, integration failures, and unexpected behavior through evidence-first root-cause analysis. Use for broken behavior before proposing or applying a fix, including when the cause seems obvious or previous fixes failed.
---

# Sonata Fix

Fix the root cause, not the symptom. Scale evidence and validation to the bug's actual risk; do not guess or force test ceremony.

1. Run `./scripts/check-sonata.sh --ready`. If pending, use `$sonata-retrofit` for an established codebase or `$sonata-setup` otherwise, then resume the bug.
2. Establish evidence before editing:
   - Read the complete error, stack trace, logs, and reported behavior.
   - Reproduce at the narrowest public seam. If intermittent, gather bounded evidence instead of guessing.
   - Inspect recent diffs, commits, environment, configuration, dependencies, and callers of the suspected shared seam.
   - In multi-component flows, observe inputs, outputs, configuration, and state at each boundary to find where behavior first diverges.
   - Trace bad data or state backward to its source.
3. Compare the broken path with a working repository pattern. Read relevant references completely and list material differences and assumptions.
4. State one falsifiable hypothesis: "X is the root cause because Y." Test it with the smallest probe or change, one variable at a time. Remove disproved probes before forming a new hypothesis.
5. Once confirmed, choose evidence dynamically from `docs/quality.md`:
   - For a trivial deterministic defect, preserve a minimal reproduction or cheap public check.
   - For branching or regression-prone behavior, add one focused regression test at the public seam.
   - For persistence, security, concurrency, money, or external contracts, use focused integration evidence.
6. Follow `$sonata-implement` for one root-cause fix. Avoid unrelated refactors and while-here cleanup.
7. Re-run the reproduction, relevant checks, and a credible sibling or regression path. Update durable docs, fixtures, logs, or harness rules when the bug exposed missing context.
8. If an attempt fails, return to evidence with the new information. After three failed hypotheses or fixes, stop and use `$sonata-grill` to question the architecture with the user before trying again.
9. Use `$sonata-review` for broad, risky, or explicitly requested fixes.

If the cause is external or environmental, document the evidence and add appropriate handling or observability instead of pretending the application caused it.
