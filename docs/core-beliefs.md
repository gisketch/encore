# Core Beliefs

## Repository Is The System Of Record

Agents can use only inspectable context. Keep product intent, boundaries, commands, decisions, and recurring lessons in versioned repo files. Trackers coordinate work; they do not replace canonical specs.

## Agent Legibility Is A Design Goal

Code should be easy for a new human or agent to navigate without hidden context. Keep entry points, ownership, interfaces, commands, logs, and runtime behavior discoverable from the repository. Use `AGENTS.md` as a short map to focused detail.

## Boring Beats Clever

Prefer existing patterns, standard library features, stable dependencies, and explicit code. Add abstractions only for a real current need. A maintainer should understand the solution without reconstructing tricks or speculative flexibility.

## Structure Shows Ownership

Organize directories by feature, domain, or responsibility. Keep public interfaces narrow and dependency directions obvious. Do not mix unrelated behavior in one file or create generic dumping grounds such as oversized `utils`, `helpers`, or `common` modules.

## Files Stay Cohesive And Small

Treat 250–350 lines as a healthy working range for a cohesive source file, not a target to pad toward. Above 350 physical lines is a design smell unless the file is generated, migrated, vendored, or framework-required. Split at real feature, responsibility, or interface boundaries—not arbitrary line ranges. Keep required exceptions out of product code: add `repo/relative/path<TAB>reason` to `.sonata/large-files.txt`.

## Guides Need Sensors

Instructions increase first-pass quality. Tests, builds, linters, runtime evidence, and reviews enable self-correction. Prefer fast deterministic checks, and make failures explain what the agent should fix.

## Evidence Follows Risk

Prototype and trivial work need one cheap check, not a test suite. Branching behavior needs a narrow public-seam test. Persistence, concurrency, security, and contracts need focused integration evidence.

## Validate Boundaries, Not Guesses

Parse and validate data at trust boundaries. Use typed or documented contracts where the project supports them. Never build behavior on guessed API shapes, database fields, permissions, or runtime assumptions.

## Humans Steer

Humans own intent, priorities, and hard tradeoffs. Agents inspect, implement, validate, review, and feed repeated failures back into the harness.

## Garbage Collect Continuously

Delete stale docs, duplicate rules, dead skills, and obsolete scaffolding in small passes. Do not wait for a heroic cleanup.
