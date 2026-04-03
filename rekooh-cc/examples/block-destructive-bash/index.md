# Example: Block Destructive Bash Commands

## Problem

Prevent Claude from executing dangerous shell commands like `rm -rf /`, `git push --force`, or `git reset --hard` that could cause irreversible damage.

## Solution

A PreToolUse hook that inspects Bash commands before execution and blocks known destructive patterns.

### `.claude/hooks/guard-destructive.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Exit early if no command
if [ -z "$COMMAND" ]; then
  exit 0
fi

# Destructive patterns to block
PATTERNS=(
  'rm\s+-rf\b'
  'rm\s+-r\s+/'
  'git\s+push\s+--force\b'
  'git\s+push\s+-f\b'
  'git\s+reset\s+--hard\b'
  'git\s+clean\s+-f'
  'git\s+checkout\s+\.\s*$'
  'git\s+checkout\s+--\s+\.\s*$'
  'git\s+restore\s+\.\s*$'
  'git\s+branch\s+-D\b'
  'mkfs\.'
  ':\s*>\s*/'
  'dd\s+if=.+of=/dev/'
)

for PATTERN in "${PATTERNS[@]}"; do
  if echo "$COMMAND" | grep -qE "$PATTERN"; then
    echo "BLOCKED: destructive command detected (matches: $PATTERN)" >&2
    exit 2
  fi
done

exit 0
```

Make it executable:

```bash
chmod +x .claude/hooks/guard-destructive.sh
```

## Settings registration

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/guard-destructive.sh"
          }
        ]
      }
    ]
  }
}
```

## Validation

Test that dangerous commands are blocked:

```bash
echo '{"tool_name":"Bash","tool_input":{"command":"rm -rf /"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-destructive.sh
echo "Exit code: $?"
# Expected: Exit code: 2 (blocked)
# Expected stderr: BLOCKED: destructive command detected
```

Test that safe commands pass:

```bash
echo '{"tool_name":"Bash","tool_input":{"command":"echo hello"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-destructive.sh
echo "Exit code: $?"
# Expected: Exit code: 0 (allowed)
```

## Limitations

Regex-based matching has known bypass vectors:
- Path-qualified binaries: `/bin/rm -rf /`
- Split flags: `rm -r -f`
- Shell indirection: `eval "rm -rf /"`, `$(echo rm) -rf /`
- Backslash line continuation

For comprehensive coverage, consider the typed runtime version which strips string literals before matching.
