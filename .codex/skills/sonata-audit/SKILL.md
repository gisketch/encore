---
name: sonata-audit
description: Audit an existing Sonata repository for full-repository security, secrets, quality, and SCC complexity debt, then write a prioritized remediation plan without changing code. Use for code-health audits, security cleanup planning, complexity hotspot reviews, or intentional legacy-debt reduction outside routine changed-code gates.
---

# Sonata Audit

Inspect the complete repository and turn verified findings into an actionable plan. Stay advisory.

1. Read `.sonata/quality-gates.json`, `.sonata/skylos.toml`, `docs/quality.md`, architecture, active plans, and Git state. If setup is pending, mention it but continue the read-only audit.
2. Require exact configured versions for enabled tools. If SCC or Skylos is disabled or unavailable, report that coverage gap and direct the user to `$sonata-setup`; do not install anything implicitly.
3. Run full-repository scans without changed-code filtering:
   - SCC: `scc --by-file --format json .`
   - Skylos: `skylos --config-file .sonata/skylos.toml --danger --secrets --quality --ai-defects --format json --no-provenance .`
4. Treat scanner output as evidence, not truth. Inspect the referenced code before accepting critical or high-impact findings. Deduplicate equivalent findings and separate false positives from real debt.
5. Rank work in this order: exploitable security and exposed secrets; high-confidence security weaknesses; complexity or nesting hotspots; other quality and AI-defect findings.
6. Split quick local repairs from larger boundary or module refactors. Preserve explicit suppressions only when their reason remains valid; never add a suppression merely to make the report clean.
7. Create or update `docs/exec-plans/active/YYYY-MM-DD-repository-health.md` with Goal, Acceptance Criteria, Context Links, ordered Steps, Validation, Decision Log, and Progress Log. Include file paths, evidence, dependencies, and the narrowest verification command for each slice.
8. Stop after the plan. Do not edit application code, change thresholds, or publish tickets without approval. Offer `$sonata-work` to implement the first approved slice.
