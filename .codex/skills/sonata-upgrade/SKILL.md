---
name: sonata-upgrade
description: Safely upgrade an existing Sonata project through the deterministic CLI updater, then refresh only stale or missing repository documentation. Use when a Sonata manifest, managed skills, checks, workflow block, or docs may be behind the latest Sonata version and local customizations must not be overwritten.
---

# Sonata Upgrade

Use the CLI for ownership-safe file changes and use judgment only for project-owned documentation.

1. Inspect Git status, `.sonata/manifest.json`, current Sonata files, and local customizations. Do not require a clean tree, but keep unrelated changes out of the upgrade.
2. Run `npx github:gisketch/sonata update --dry-run`. Explain planned writes, deletes, preserved legacy files, and conflicts before changing anything.
3. If any conflict exists, stop. Never force, overwrite, delete, or automatically merge a modified Sonata-owned file. Help the user inspect and choose a deliberate reconciliation.
4. If the plan is safe, ask for confirmation, then run `npx github:gisketch/sonata update`.
5. Reread the updated `AGENTS.md`, skills, manifest, and checks because the upgrade may have changed this workflow.
6. Compare project-owned docs with the actual codebase. Update only stale or missing facts, links, commands, boundaries, or milestone context; leave correct custom documentation untouched. Ask only when a real product decision is missing.
7. Run `./scripts/check-sonata.sh` and relevant verified project checks. If readiness is pending, offer `$sonata-retrofit` for an established codebase or `$sonata-setup` for greenfield work.

The skill never replaces the updater's checksums, preflight, transactions, or rollback.
