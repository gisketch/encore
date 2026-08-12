# encore

Agent-legible project built with Sonata

## Start

1. Read `AGENTS.md` and `docs/index.md`.
2. Install dependencies with `npm install`.
3. Launch the macOS shell with `npm run tauri dev`.

The current shell is an honest interface preview. Screen capture, rolling
storage, packaging, and the global shortcut are not implemented yet.

## Verify

```sh
npm run check
npm run build
(cd src-tauri && cargo fmt --check)
(cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings)
(cd src-tauri && cargo test)
./scripts/check-sonata.sh --ready
```

## Workflow

- Start any feature, fix, or chore with `$sonata-work`.
- Diagnose broken behavior through `$sonata-fix` before changing code.
- Grill unclear designs with `$sonata-grill`.
- Keep canonical specs under `docs/specs/`.
- Slice broad work with `$sonata-tickets`.
- Implement with risk-based checks through `$sonata-implement`.
- Review Standards, Spec, and Behavior with `$sonata-review`.
- Audit full-repository security and complexity debt with `$sonata-audit`.

Issue trackers are optional coordination. Repository docs and `.agents/skills/` remain canonical.

## Optional quality gates

Use `$sonata-setup` to enable SCC, Skylos, or both. Disabled tools are never required. Enabled tools become strict changed-code checks through `node scripts/check-quality-gates.mjs`; use `$sonata-audit` to plan cleanup of untouched repository debt.
