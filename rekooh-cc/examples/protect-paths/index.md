# Example: Protect Sensitive File Paths

## Problem

Prevent Claude from reading or modifying sensitive files like `.env`, credentials, or settings files.

## Solution

A PreToolUse hook that checks file paths across Bash, Write, Edit, and Read tools and blocks access to protected paths.

### `.claude/hooks/guard-paths.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name')

# Protected paths — add your own
PROTECTED_PATHS=(
  ".env"
  "secrets/"
  "credentials.json"
  ".claude/settings.json"
)

# Extract the relevant path based on tool type
case "$TOOL_NAME" in
  Bash)
    TARGET=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
    # For Bash, check if the command string mentions any protected path
    for PROTECTED in "${PROTECTED_PATHS[@]}"; do
      if echo "$TARGET" | grep -qF "$PROTECTED"; then
        echo "BLOCKED: command references protected path: $PROTECTED" >&2
        exit 2
      fi
    done
    ;;
  Write|Edit|Read)
    TARGET=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
    for PROTECTED in "${PROTECTED_PATHS[@]}"; do
      if echo "$TARGET" | grep -qF "$PROTECTED"; then
        echo "BLOCKED: operation on protected path: $PROTECTED" >&2
        exit 2
      fi
    done
    ;;
esac

exit 0
```

Make it executable:

```bash
chmod +x .claude/hooks/guard-paths.sh
```

## Settings registration

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Write|Edit|Read",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/guard-paths.sh"
          }
        ]
      }
    ]
  }
}
```

Note the matcher covers all four tool types that could access files.

## Validation

Test that protected paths are blocked:

```bash
# Test Write to .env
echo '{"tool_name":"Write","tool_input":{"file_path":".env","content":"SECRET=abc"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-paths.sh
echo "Exit code: $?"
# Expected: Exit code: 2

# Test Bash command referencing .env
echo '{"tool_name":"Bash","tool_input":{"command":"cat .env"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-paths.sh
echo "Exit code: $?"
# Expected: Exit code: 2

# Test safe file
echo '{"tool_name":"Write","tool_input":{"file_path":"src/app.ts","content":"console.log()"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-paths.sh
echo "Exit code: $?"
# Expected: Exit code: 0
```

## Customization

Edit the `PROTECTED_PATHS` array to add project-specific paths:

```bash
PROTECTED_PATHS=(
  ".env"
  ".env.local"
  "secrets/"
  "config/production.yml"
  "terraform.tfstate"
)
```
