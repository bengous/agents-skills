# Testing — Validate Hook Behavior

Test hooks before registering them to catch issues early.

## Manual testing with pipe

The simplest way to test any hook is piping JSON to it and checking the exit code:

```bash
# Test a bash hook
echo '{"tool_name":"Bash","tool_input":{"command":"rm -rf /"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bash .claude/hooks/guard-destructive.sh
echo "Exit code: $?"
```

```bash
# Test a bun hook
echo '{"tool_name":"Bash","tool_input":{"command":"echo hello"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bun .claude/hooks/guard-safe.ts
echo "Exit code: $?"
```

**Exit code semantics:**
- `0` — Allow (hook approves the action)
- `1` — Error (hook failed; Claude logs warning, action proceeds)
- `2` — Block (hook explicitly blocks the action; stderr contains the reason)

## Automated validation

Run the validation script to check all registered hooks:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/validate-hooks.ts
```

The script:
1. Reads `.claude/settings.json` to find all registered hooks
2. Verifies each hook command's file exists and is executable
3. Runs each hook with a synthetic test payload for its event type
4. Reports exit codes and any unexpected behavior

## Testing with the typed runtime

The opinionated typed runtime provides a test harness and payload factories purpose-built for hook testing.

### Test harness — `testHook`

```typescript
import { testHook } from "./_lib/testing";

const hook = testHook(".claude/hooks/src/guard-destructive.ts");

const result = await hook.run({
  session_id: "test",
  transcript_path: "/tmp/t",
  cwd: "/tmp",
  permission_mode: "default",
  hook_event_name: "PreToolUse",
  tool_name: "Bash",
  tool_input: { command: "rm -rf /" },
});

console.log(result.exitCode); // 2 (blocked)
console.log(result.stderr);   // "Blocked: destructive command"
```

`testHook(path)` returns an object with:
- `run(input, env?)` — Serializes the input as JSON, pipes it to the hook, returns `{ exitCode, stdout, stderr, json() }`
- `runRaw(stdin, env?)` — Same but with raw string stdin

### Payload factories — `createPayload`

```typescript
import { createPayload } from "./_lib/testing";

// Full PreToolUse payload with sensible defaults
const payload = createPayload("PreToolUse", {
  tool_name: "Bash",
  tool_input: { command: "git push --force" },
});
// Returns complete payload with session_id, transcript_path, cwd, etc. pre-filled
```

`createPayload(event, overrides?)` produces a complete, valid payload for any of the 18 event types. Override only the fields relevant to your test.

### Example test file

```typescript
import { describe, expect, test } from "bun:test";
import { createPayload } from "./_lib/testing";
import { testHook } from "./_lib/testing";

const hook = testHook(".claude/hooks/src/guard-destructive.ts");

describe("guard-destructive", () => {
  test("blocks rm -rf", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Bash",
        tool_input: { command: "rm -rf /" },
      })
    );
    expect(result.exitCode).toBe(2);
    expect(result.stderr).toContain("destructive");
  });

  test("allows safe commands", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Bash",
        tool_input: { command: "echo hello" },
      })
    );
    expect(result.exitCode).toBe(0);
  });
});
```

## Troubleshooting

| Symptom | Likely cause |
|---------|-------------|
| Exit code 1 instead of 0 or 2 | Hook script has a runtime error (check stderr) |
| Hook never blocks | Matcher in settings.json doesn't match the test tool name |
| Timeout | Hook command is waiting for input or stuck; check for missing stdin handling |
| JSON parse error | Stdin isn't valid JSON; ensure the test payload is properly serialized |
| Permission denied | Shell script not executable (`chmod +x`) |
