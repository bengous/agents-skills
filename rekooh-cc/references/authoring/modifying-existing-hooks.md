# Modifying Existing Hooks

How to safely change hooks that are already registered and running.

## Before modifying

1. **Read the current hook** — Understand what it does, what event it handles, and what its exit conditions are.

2. **Check the settings registration** — Find the hook in `.claude/settings.json` to understand its matcher and timeout configuration.

3. **Understand idempotency** — Hooks should produce the same result for the same input. If your change introduces state (files, environment variables), ensure it doesn't break this property.

## Safe modification workflow

### 1. Test the current behavior

Before changing anything, verify the hook works as expected with known inputs:

```bash
# Create a test payload
echo '{"tool_name":"Bash","tool_input":{"command":"echo hello"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-destructive.sh
echo "Exit: $?"
```

### 2. Make the change

Edit the hook file. Common modifications:
- Adding new patterns to a guard list
- Changing the response from allow to block (or vice versa)
- Adding context to responses
- Changing the matcher scope in settings

### 3. Test the modified behavior

Re-run the same test payloads plus new ones that exercise the change:

```bash
# Test the new pattern
echo '{"tool_name":"Bash","tool_input":{"command":"new-pattern-here"},...}' | bash .claude/hooks/guard-destructive.sh
echo "Exit: $?"
```

### 4. Validate the full setup

```bash
bun ~/.claude/skills/rekooh-cc/scripts/validate-hooks.ts
```

## Common modifications

### Add patterns to a guard hook

If using a pattern list, add to the array/regex:

```bash
# Before
if echo "$COMMAND" | grep -qE 'rm\s+-rf'; then

# After — added git push --force
if echo "$COMMAND" | grep -qE 'rm\s+-rf|git\s+push\s+--force'; then
```

### Change the matcher scope

In `.claude/settings.json`, update the `matcher` field:

```json
// Before: only Bash
{ "matcher": "Bash" }

// After: Bash and shell-related tools
{ "matcher": "Bash|Terminal" }
```

### Add a second hook for the same event

Add another entry to the event's array in settings. Both run in parallel:

```json
{
  "hooks": {
    "PreToolUse": [
      { "type": "command", "command": "bash .claude/hooks/guard-destructive.sh", "matcher": "Bash" },
      { "type": "command", "command": "bash .claude/hooks/guard-paths.sh", "matcher": "Write|Edit" }
    ]
  }
}
```

### Switch from standalone to typed runtime

1. Bootstrap the typed runtime ([bootstrap](../bootstrap/index.md))
2. Rewrite the hook using `defineHook` ([typed-runtime-patterns](typed-runtime-patterns.md))
3. Update the command in settings from `bash .claude/hooks/old.sh` to `bun .claude/hooks/src/new.ts`
4. Remove the old hook file after validating the new one

## Risks to watch for

| Risk | Mitigation |
|------|------------|
| Breaking an active guard | Test with both allowed and blocked inputs before and after |
| Duplicate registrations | Use `patch-settings.ts` for idempotent updates |
| Timeout changes | Only increase timeouts if the hook genuinely needs more time |
| Removing a hook | Verify no other hooks depend on its behavior (e.g., shared state) |
