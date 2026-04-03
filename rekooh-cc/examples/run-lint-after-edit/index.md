# Example: Run Linter After File Edits

## Problem

Automatically run a linter or formatter after Claude edits files, catching style violations and errors immediately rather than discovering them later.

## Solution

A PostToolUse hook that runs after Write/Edit operations and checks the modified file with a linter.

### `.claude/hooks/lint-after-edit.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Skip if no file path
if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# Only lint specific file types
case "$FILE_PATH" in
  *.ts|*.tsx|*.js|*.jsx|*.mjs)
    ;;
  *)
    exit 0
    ;;
esac

# Skip if file doesn't exist (was deleted)
if [ ! -f "$FILE_PATH" ]; then
  exit 0
fi

# Auto-format first (non-blocking)
npx biome format --write -- "$FILE_PATH" 2>/dev/null || true

# Then lint (blocking on errors)
LINT_OUTPUT=$(npx biome lint --diagnostic-level=error -- "$FILE_PATH" 2>&1) || {
  echo "Lint errors in $FILE_PATH:" >&2
  echo "$LINT_OUTPUT" >&2
  exit 2
}

exit 0
```

Make it executable:

```bash
chmod +x .claude/hooks/lint-after-edit.sh
```

## Settings registration

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/lint-after-edit.sh"
          }
        ]
      }
    ]
  }
}
```

## Validation

Test with a valid TypeScript file:

```bash
# Create a test file
echo 'const x: number = 42;' > /tmp/test-lint.ts

# Simulate a PostToolUse event
echo '{"tool_name":"Write","tool_input":{"file_path":"/tmp/test-lint.ts","content":"const x = 42;"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PostToolUse"}' | bash .claude/hooks/lint-after-edit.sh
echo "Exit code: $?"
# Expected: Exit code: 0 (passes lint)
```

Test with non-JS file (should be skipped):

```bash
echo '{"tool_name":"Write","tool_input":{"file_path":"README.md","content":"# Hello"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PostToolUse"}' | bash .claude/hooks/lint-after-edit.sh
echo "Exit code: $?"
# Expected: Exit code: 0 (skipped, not a JS/TS file)
```

## Adapting to other linters

Replace the biome commands with your project's linter:

**ESLint:**
```bash
npx eslint --fix "$FILE_PATH" 2>/dev/null || true
LINT_OUTPUT=$(npx eslint "$FILE_PATH" 2>&1) || { echo "$LINT_OUTPUT" >&2; exit 2; }
```

**Prettier + ESLint:**
```bash
npx prettier --write "$FILE_PATH" 2>/dev/null || true
LINT_OUTPUT=$(npx eslint "$FILE_PATH" 2>&1) || { echo "$LINT_OUTPUT" >&2; exit 2; }
```

**Ruff (Python):**
```bash
ruff format "$FILE_PATH" 2>/dev/null || true
LINT_OUTPUT=$(ruff check "$FILE_PATH" 2>&1) || { echo "$LINT_OUTPUT" >&2; exit 2; }
```
