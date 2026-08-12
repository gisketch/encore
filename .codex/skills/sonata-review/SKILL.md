---
name: sonata-review
description: Review Sonata changes against repository standards, an originating spec, and observable behavior. Use for pull requests, branches, working-tree changes, milestone checks, or explicit code-review requests.
---

# Sonata Review

Review only. Do not edit unless asked.

If Sonata setup is pending, mention it but continue; review remains available for diagnosis and setup changes.

1. Pin the diff. Use the user's fixed point; otherwise review staged and unstaged changes against `HEAD`.
2. Read `AGENTS.md`, relevant architecture, `docs/quality.md`, and the originating spec or ticket.
3. Run the narrowest cheap computational checks that clarify findings.
4. Report findings separately:
   - Standards: documented boundaries, duplication, file-size pressure, maintainability, security, and scope discipline.
   - Spec: missing, partial, incorrect, or unrequested behavior. Quote the requirement.
   - Behavior: insufficient evidence, wrong public behavior, regressions, or risky untested seams.
5. Rank findings by impact: P0 data/security loss, P1 broken required behavior, P2 maintainability or edge-case risk, P3 minor cleanup.
6. Cite exact file and line. Explain consequence and smallest credible fix.
7. Skip style issues already enforced by tooling and tests that add no behavior confidence.

If no spec exists, say so and keep that axis separate. No mandatory sub-agents; use them only when independent context materially improves a broad review.
