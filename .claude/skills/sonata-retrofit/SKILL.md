---
name: sonata-retrofit
description: Ground Sonata in an existing codebase through deep repository inspection before interviewing the user. Use after running Sonata init in an established project when its real architecture, commands, domain, constraints, and existing documentation must replace generic harness placeholders without overwriting useful knowledge.
---

# Sonata Retrofit

Convert an existing codebase into a repository-grounded Sonata project. Read deeply before asking anything.

1. Verify Sonata files exist; otherwise direct the user to run `npx github:gisketch/sonata init .` first.
2. Inventory the whole repository: source and workspace layout, entry points, runtime boundaries, domain modules, data ownership, package/build files, tests, scripts, deployment/configuration, remotes, Git history, and existing docs.
3. Trace the main user and runtime paths far enough to distinguish real behavior from filenames or placeholders. Build an evidence-backed project map before interviewing.
4. Read `.codex/skills/sonata-setup/SKILL.md` and satisfy the same readiness contract using discovered facts. Skip questions already answered by the repository and questions irrelevant to this project.
5. Ask exactly one unresolved decision per turn. Give a recommendation grounded in the inspected code and explain its main tradeoff.
6. Update project brief, architecture, quality commands, issue-tracker configuration, and indexes with verified facts. Preserve, link, or consolidate useful existing docs; never replace knowledge merely to match Sonata's template.
7. Run the harness check and narrowest verified project checks. Mark setup ready only after the repository map and current milestone are credible.
8. Offer `$sonata-work` for the next real change.

Do not restructure application code during retrofit. Do not invent architecture, commands, or product intent to fill blanks.
