#!/bin/sh
# PreToolUse(Bash) — CLAUDE.md doc rule: code changes must ship with doc updates.
# Blocks `git commit` when src/ or src-tauri/ is staged without any doc staged.
# Bypass: prefix the commit command with SKIP_DOCS=1.

input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // ""')

case "$cmd" in
  *"git commit"*) ;;
  *) exit 0 ;;
esac
case "$cmd" in
  *SKIP_DOCS=1*) exit 0 ;;
esac

staged=$(git diff --cached --name-only 2>/dev/null) || exit 0

printf '%s\n' "$staged" | grep -qE '^(src|src-tauri)/' || exit 0
printf '%s\n' "$staged" | grep -qE '^(docs/|DEVLOG\.md|CLAUDE\.md|README\.md)' && exit 0

echo "Regola CLAUDE.md: file in src/ o src-tauri/ in stage senza aggiornamento docs/, DEVLOG.md, CLAUDE.md o README.md. Aggiorna e stagea il doc corrispondente, oppure premetti SKIP_DOCS=1 al comando per saltare il controllo." >&2
exit 2
