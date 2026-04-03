#!/usr/bin/env bash
set -euo pipefail

# Wrapper for TypeScript hooks using the typed runtime.
#
# Hooks execute from the project root, but TS hooks need their own
# node_modules (for claude-hooks, effect, etc.). This script:
# 1. Captures stdin (hook payload)
# 2. cd's to the hooks directory (where package.json lives)
# 3. Runs the hook file with bun, piping the original stdin
#
# Usage in .claude/settings.json:
#   { "hooks": { "PreToolUse": [{ "hooks": [{ "type": "command", "command": ".claude/hooks/run-hook.sh src/guard.ts" }] }] } }

HOOKS_DIR="$(cd "$(dirname "$0")" && pwd)"
HOOK_FILE="${1:?Usage: run-hook.sh <hook-file>}"

# Read stdin once (it's consumed on first read)
INPUT=$(cat)

cd "$HOOKS_DIR"
echo "$INPUT" | exec bun "$HOOK_FILE"
