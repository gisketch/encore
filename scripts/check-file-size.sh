#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

failed=0
is_allowed_large_file() {
  local relative="${1#./}"
  [[ -f .sonata/large-files.txt ]] &&
    awk -F '\t' -v target="$relative" '$1 == target && length($2) > 0 { found=1 } END { exit !found }' .sonata/large-files.txt
}

while IFS= read -r -d '' file; do
  case "$file" in
    */.git/*|*/node_modules/*|*/vendor/*|*/dist/*|*/build/*|*/coverage/*|*/migrations/*|*/.tmp-sonata-*/*|*.generated.*|*.g.cs) continue ;;
  esac
  if is_allowed_large_file "$file"; then continue; fi
  lines="$(wc -l < "$file" | tr -d ' ')"
  if (( lines > 350 )); then
    printf 'source file exceeds 350 lines: %s (%s)\n' "$file" "$lines"
    failed=1
  fi
done < <(find . -type f \( \
  -name '*.js' -o -name '*.jsx' -o -name '*.ts' -o -name '*.tsx' -o \
  -name '*.py' -o -name '*.cs' -o -name '*.rs' -o -name '*.go' -o \
  -name '*.java' -o -name '*.kt' -o -name '*.kts' -o -name '*.sh' -o \
  -name '*.bash' -o -name '*.zsh' -o -name '*.c' -o -name '*.h' -o \
  -name '*.cc' -o -name '*.cpp' -o -name '*.hpp' -o -name '*.rb' -o \
  -name '*.php' -o -name '*.swift' -o -name '*.m' -o -name '*.mm' -o \
  -name '*.vue' -o -name '*.svelte' -o -name '*.scala' -o -name '*.ex' -o \
  -name '*.exs' -o -name '*.erl' -o -name '*.fs' -o -name '*.fsx' \
\) -print0)

if (( failed )); then
  printf 'split by feature, responsibility, or interface boundary; do not slice arbitrarily\n'
  exit 1
fi

printf 'file size ok\n'
