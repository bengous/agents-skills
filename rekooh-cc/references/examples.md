# Examples

These examples are deliberately small. For exact current fields and advanced
handler types, check `https://code.claude.com/docs/en/hooks`.

## Block Destructive Bash

`.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/block-destructive.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

`.claude/hooks/block-destructive.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

payload="$(cat)"
command="$(jq -r '.tool_input.command // empty' <<<"$payload")"

if [[ "$command" =~ (^|[[:space:]])rm[[:space:]]+-rf($|[[:space:]]) ]]; then
  echo "Blocked: rm -rf requires explicit human execution" >&2
  exit 2
fi
```

## Lint After Edits

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit",
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/post-edit-check.sh",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

```bash
#!/usr/bin/env bash
set -euo pipefail

if [[ -f package.json ]] && jq -e '.scripts.check' package.json >/dev/null; then
  bun run check
fi
```

For validation hooks like this, fail open if noisy diagnostics would block useful
work too often.

## Session Context

Use `SessionStart` when stable project context is helpful and cheap to compute:

```bash
#!/usr/bin/env bash
set -euo pipefail

if [[ -f package.json ]]; then
  jq -n --arg context "package.json exists; inspect scripts before running tools" \
    '{hookSpecificOutput:{additionalContext:$context}}'
fi
```

## Register A Command Hook

```bash
bun <skill-root>/scripts/patch-settings.ts \
  --settings .claude/settings.json \
  --event PreToolUse \
  --matcher Bash \
  --command ".claude/hooks/block-destructive.sh" \
  --timeout 30
```
