# Create a Hook with the Typed Runtime

Step-by-step guide to building a hook using `defineHook`.

## Prerequisites

- Bun runtime installed
- Typed runtime scaffolded into the project (see [bootstrap](../bootstrap/index.md))

## Step 1: Create the hook file

Create a new `.ts` file in your hooks source directory:

```typescript
// .claude/hooks/src/guard-force-push.ts
import { defineHook } from "../_lib/core";
```

## Step 2: Choose the event

Pick the event your hook handles. This determines:
- What input fields are available
- What response builders you get
- What matcher field applies (if any)

```typescript
defineHook("PreToolUse", {
  // ...
});
```

See [events reference](../events/index.md) for all 18 events.

## Step 3: Configure the matcher (optional)

If the event supports a matcher field, use it to filter which instances your hook handles:

```typescript
defineHook("PreToolUse", {
  matcher: "Bash",  // Only run for Bash tool uses
  // ...
});
```

String matchers are anchored (`^...$`). Use a RegExp for patterns:

```typescript
matcher: /^(Bash|Terminal)$/,
```

## Step 4: Write the check function

The `check` function receives typed input and the appropriate response builders:

```typescript
defineHook("PreToolUse", {
  matcher: "Bash",
  check(input, respond) {
    const command = (input.tool_input as Record<string, string>).command ?? "";

    // Block force pushes
    if (/git\s+push\s+.*--force/.test(command)) {
      return respond.deny("Force push is blocked by project policy");
    }

    return respond.allow();
  },
});
```

The `input` object is fully typed for the chosen event. For PreToolUse:
- `input.tool_name` — string
- `input.tool_input` — Record<string, unknown>
- `input.tool_use_id` — string | undefined
- `input.session_id`, `input.cwd`, etc. — common fields
- `input._raw` — the raw untyped JSON object

The `respond` object provides the response builders for the event's family. For PreToolUse:
- `respond.allow()` — allow the tool use
- `respond.deny(reason)` — block with a reason
- `respond.ask(reason?)` — ask the user for permission
- `respond.allowWithInput(updatedInput, context?)` — allow with modified input

## Step 5: Set a failure policy (optional)

```typescript
defineHook("PreToolUse", {
  matcher: "Bash",
  failurePolicy: "fail-closed",  // Block on error (for security-critical hooks)
  check(input, respond) {
    // ...
  },
});
```

Default is `"fail-open"` — if the hook crashes, the action proceeds.

## Step 6: Register in settings

Add the hook to `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "type": "command",
        "command": "bun .claude/hooks/src/guard-force-push.ts",
        "matcher": "Bash"
      }
    ]
  }
}
```

Or use the patch script:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/patch-settings.ts
```

## Step 7: Test

Using the test harness:

```typescript
// .claude/hooks/tests/guard-force-push.test.ts
import { describe, expect, test } from "bun:test";
import { testHook } from "../_lib/testing";
import { createPayload } from "../_lib/testing";

const hook = testHook(".claude/hooks/src/guard-force-push.ts");

describe("guard-force-push", () => {
  test("blocks force push", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Bash",
        tool_input: { command: "git push --force origin main" },
      })
    );
    expect(result.exitCode).toBe(2);
    expect(result.stderr).toContain("Force push");
  });

  test("allows normal push", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Bash",
        tool_input: { command: "git push origin main" },
      })
    );
    expect(result.exitCode).toBe(0);
  });

  test("skips non-Bash tools", async () => {
    const result = await hook.run(
      createPayload("PreToolUse", {
        tool_name: "Write",
        tool_input: { file_path: "/tmp/test.txt", content: "hello" },
      })
    );
    expect(result.exitCode).toBe(0); // Matcher short-circuit
  });
});
```

Run tests:

```bash
bun test .claude/hooks/tests/guard-force-push.test.ts
```

## Complete example

```typescript
// .claude/hooks/src/guard-force-push.ts
import { defineHook } from "../_lib/core";

defineHook("PreToolUse", {
  matcher: "Bash",
  failurePolicy: "fail-closed",
  check(input, respond) {
    const command = (input.tool_input as Record<string, string>).command ?? "";

    if (/git\s+push\s+.*--force/.test(command)) {
      return respond.deny("Force push is blocked by project policy");
    }

    if (/git\s+reset\s+--hard/.test(command)) {
      return respond.deny("Hard reset is blocked by project policy");
    }

    return respond.allow();
  },
});
```

Settings registration:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "type": "command",
        "command": "bun .claude/hooks/src/guard-force-push.ts",
        "matcher": "Bash"
      }
    ]
  }
}
```
