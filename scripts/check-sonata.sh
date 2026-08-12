#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

skills=()
for skill_dir in .agents/skills/sonata-*; do skills+=("${skill_dir##*/}"); done
required=(
  AGENTS.md README.md .gitignore .sonata/manifest.json .sonata/large-files.txt
  .sonata/quality-gates.json .sonata/skylos.toml
  docs/index.md docs/project-brief.md docs/core-beliefs.md docs/quality.md
  docs/architecture/index.md docs/specs/README.md docs/exec-plans/README.md
  scripts/check-sonata.sh scripts/check-file-size.sh scripts/check-quality-gates.mjs
  scripts/install-quality-tools.mjs scripts/quality-gates.mjs
)

for skill in "${skills[@]}"; do
  required+=(".agents/skills/$skill/SKILL.md")
done

manifest_has() { grep -q "\"$1\"" .sonata/manifest.json 2>/dev/null; }

if manifest_has codex; then
  for skill in "${skills[@]}"; do required+=(".codex/skills/$skill/SKILL.md"); done
fi
if manifest_has copilot; then
  required+=(.github/copilot-instructions.md)
fi
if manifest_has claude; then
  required+=(CLAUDE.md)
  for skill in "${skills[@]}"; do required+=(".claude/skills/$skill/SKILL.md"); done
fi
if manifest_has pi; then
  required+=(.pi/settings.json)
  for skill in "${skills[@]}"; do required+=(".pi/skills/$skill/SKILL.md"); done
fi

missing=0
for file in "${required[@]}"; do
  if [[ ! -s "$file" ]]; then
    printf 'missing or empty: %s\n' "$file"
    missing=1
  fi
done
(( missing == 0 )) || exit 1

grep -q '"schema": 2' .sonata/manifest.json || {
  printf 'manifest schema must be 2\n'
  exit 1
}

./scripts/check-file-size.sh
node scripts/check-quality-gates.mjs

if [[ "${1:-}" == "--ready" ]]; then
  node -e '
    const fs = require("fs");
    const manifest = JSON.parse(fs.readFileSync(".sonata/manifest.json", "utf8"));
    if (manifest.setup?.status !== "ready") {
      console.error("Sonata setup is incomplete. Run $sonata-setup first.");
      process.exit(1);
    }
  '
fi

printf 'sonata ok\n'
