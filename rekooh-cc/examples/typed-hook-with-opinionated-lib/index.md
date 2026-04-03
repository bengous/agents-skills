# Example: Typed Hook with Opinionated Runtime

## Problem

Build a type-safe PreToolUse guard that blocks destructive bash commands, using the typed runtime's `defineHook` for compile-time guarantees on event schemas and response types.

## Solution

A PreToolUse hook using `defineHook` with typed input decoding, response builders, and fail-closed policy.

### `.claude/hooks/src/guard-destructive.ts`

```typescript
import { defineHook } from "claude-hooks";

const BLOCKED_PATTERNS: ReadonlyArray<readonly [RegExp, string]> = [
  [/rm\s+-rf\b/, "rm -rf"],
  [/rm\s+-r\s+\//, "rm -r /"],
  [/git\s+push\s+--force\b/, "git push --force"],
  [/git\s+push\s+-f\b/, "git push -f"],
  [/git\s+reset\s+--hard\b/, "git reset --hard"],
  [/git\s+clean\s+-f/, "git clean -f"],
  [/git\s+checkout\s+\.\s*$/, "git checkout ."],
  [/git\s+checkout\s+--\s+\.\s*$/, "git checkout -- ."],
  [/git\s+restore\s+\.\s*$/, "git restore ."],
  [/git\s+branch\s+-D\b/, "git branch -D"],
];

defineHook("PreToolUse", {
  matcher: "Bash",
  failurePolicy: "fail-closed",
  check(input, respond) {
    const command = (input.tool_input as { command?: string }).command;
    if (typeof command !== "string") return respond.allow();

    for (const [pattern, label] of BLOCKED_PATTERNS) {
      if (pattern.test(command)) {
        return respond.deny(`BLOCKED: destructive command detected: ${label}`);
      }
    }

    return respond.allow();
  },
});
```

### What the typed runtime gives you

1. **`input` is fully typed** — `input.tool_name` is `string`, `input.tool_input` is `Record<string, unknown>`, `input.session_id` is `string`, etc. Invalid field access is caught at compile time.

2. **`respond` has the correct builders** — For PreToolUse, you get `.allow()`, `.deny(reason)`, `.ask(reason?)`, `.allowWithInput(input, context?)`. Other response types are a compile error.

3. **`matcher: "Bash"`** — The runtime short-circuits with allow (exit 0) for non-Bash tool uses before even decoding the input.

4. **`failurePolicy: "fail-closed"`** — If this hook crashes (parse error, runtime error), the command is blocked rather than allowed. Appropriate for security-critical guards.

## Settings registration

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bun .claude/hooks/src/guard-destructive.ts"
          }
        ]
      }
    ]
  }
}
```

## Test file

### `.claude/hooks/tests/guard-destructive.test.ts`

```typescript
import { describe, expect, test } from "bun:test";
import { testHook, createPayload } from "claude-hooks";

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
    expect(result.stderr).toContain("rm -rf");
  });

  test("blocks git push --force", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Bash",
        tool_input: { command: "git push --force origin main" },
      })
    );
    expect(result.exitCode).toBe(2);
    expect(result.stderr).toContain("git push --force");
  });

  test("allows safe commands", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Bash",
        tool_input: { command: "echo hello world" },
      })
    );
    expect(result.exitCode).toBe(0);
  });

  test("skips non-Bash tools via matcher", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Write",
        tool_input: { file_path: "/tmp/test.ts", content: "rm -rf /" },
      })
    );
    expect(result.exitCode).toBe(0);
  });
});
```

## Validation

Run the tests:

```bash
bun test .claude/hooks/tests/guard-destructive.test.ts
```

Manual test:

```bash
echo '{"tool_name":"Bash","tool_input":{"command":"rm -rf /"},"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"PreToolUse"}' | bun .claude/hooks/src/guard-destructive.ts
echo "Exit code: $?"
# Expected: Exit code: 2
# Expected stderr: BLOCKED: destructive command detected: rm -rf
```

## Prerequisites

The typed runtime must be available via the `claude-hooks` dependency in `.claude/hooks/package.json`. See [bootstrap](../../references/bootstrap/index.md) or run:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/scaffold-opinionated-hook.ts
```
