# Example: Complete Project Bootstrap

## Problem

Set up a complete hooks system in a project from scratch — from initial inspection to validated, working hooks.

## Solution

An end-to-end walkthrough using the rekooh scripts in sequence.

## Step 1: Inspect the project

Run the inspection script to understand the project's current state:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/inspect-project.ts
```

This outputs JSON with:
- Detected runtimes (bun, node, python, bash)
- Existing settings files and hooks
- Package managers and quality commands
- Recommended strategy and rationale

Review the `recommendedStrategy` field to decide your approach.

## Step 2: Bootstrap infrastructure

Create the directory structure and configuration files:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/bootstrap-hooks-infra.ts
```

After running, you'll have:
- `.claude/` directory (if it didn't exist)
- `.claude/hooks/` directory for hook scripts
- `.claude/settings.json` (if it didn't exist)

## Step 3: Scaffold a hook

### Standalone hook

For a simple standalone hook:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/scaffold-standalone-hook.ts
```

This generates a hook file based on the chosen event and strategy (bash, python, or bun).

### Opinionated typed runtime hook

For a hook using the typed runtime:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/scaffold-opinionated-hook.ts
```

This generates a typed hook entrypoint in `.claude/hooks/src/`. The runtime is accessed via the `claude-hooks` npm link in `package.json`.

## Step 4: Register in settings

Patch `.claude/settings.json` to register the new hook:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/patch-settings.ts
```

The script idempotently adds the hook registration — running it again won't create duplicates.

## Step 5: Validate

Run the validation script to verify everything works:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/validate-hooks.ts
```

The validator:
1. Reads all hook registrations from settings
2. Checks that each hook's command file exists and is executable
3. Reports results and exits non-zero on any failure

## Complete walkthrough

Here's the full sequence for a project that needs a destructive command guard:

```bash
# 1. Inspect
bun ~/.claude/skills/rekooh-cc/scripts/inspect-project.ts

# 2. Bootstrap
bun ~/.claude/skills/rekooh-cc/scripts/bootstrap-hooks-infra.ts

# 3. Create the hook file
mkdir -p .claude/hooks
cat > .claude/hooks/guard-destructive.sh << 'HOOK'
#!/usr/bin/env bash
set -euo pipefail
INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
if [ -z "$COMMAND" ]; then exit 0; fi

PATTERNS=(
  'rm\s+-rf\b'
  'git\s+push\s+--force\b'
  'git\s+reset\s+--hard\b'
  'git\s+clean\s+-f'
)

for PATTERN in "${PATTERNS[@]}"; do
  if echo "$COMMAND" | grep -qE "$PATTERN"; then
    echo "BLOCKED: destructive command ($PATTERN)" >&2
    exit 2
  fi
done
exit 0
HOOK
chmod +x .claude/hooks/guard-destructive.sh

# 4. Register in settings
bun ~/.claude/skills/rekooh-cc/scripts/patch-settings.ts

# 5. Validate
bun ~/.claude/skills/rekooh-cc/scripts/validate-hooks.ts
```

## Validation

After completing all steps, verify manually:

```bash
# Should block
echo '{"tool_name":"Bash","tool_input":{"command":"rm -rf /"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-destructive.sh
echo "Exit code: $?"  # Expected: 2

# Should allow
echo '{"tool_name":"Bash","tool_input":{"command":"ls -la"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-destructive.sh
echo "Exit code: $?"  # Expected: 0
```

## Next steps

- Add more hooks: see [authoring reference](../../references/authoring/index.md)
- Add path protection: see [protect-paths example](../protect-paths/index.md)
- Add auto-linting: see [run-lint-after-edit example](../run-lint-after-edit/index.md)
- Add stop notifications: see [stop-notification example](../stop-notification/index.md)
