# Typed Runtime Patterns

Using `defineHook` from the opinionated typed runtime for type-safe hook authoring.

## Overview

The typed runtime provides:
- **Event schema validation** — stdin JSON is decoded and validated against Effect schemas
- **Type-safe response builders** — no manual JSON construction
- **Matcher support** — optional regex filtering on the event's matcher field
- **Failure policies** — configurable behavior when the hook errors (fail-open or fail-closed)
- **Test harness** — subprocess runner and payload factories

## Basic structure

```typescript
import { defineHook } from "./_lib/core";

defineHook("PreToolUse", {
  matcher: "Bash",
  check(input, respond) {
    const command = (input.tool_input as Record<string, string>).command ?? "";

    if (/rm\s+-rf\s+\//.test(command)) {
      return respond.deny("Destructive command blocked");
    }

    return respond.allow();
  },
});
```

When this file runs, `defineHook`:
1. Reads stdin
2. Parses JSON
3. Checks the matcher field (`tool_name`) against `"Bash"`
4. If matched, decodes the input using the `PreToolUseInput` schema
5. Calls your `check` function with the typed input and response builders
6. Renders the response to stdout/stderr and exits with the correct code

## Event selection

Pass the event name as the first argument to `defineHook`:

```typescript
defineHook("PostToolUse", { ... });     // After tool execution
defineHook("Stop", { ... });            // When Claude is about to stop
defineHook("SessionStart", { ... });    // On session startup
```

The event name determines:
- Which input schema is used for decoding
- Which response builders are available
- Which matcher field applies (if any)

See [events](../events/index.md) for all 18 events and their schemas.

## Matcher config

The `matcher` field filters which instances of the event your hook handles:

```typescript
// String matcher — exact match (anchored with ^ and $)
defineHook("PreToolUse", {
  matcher: "Bash",
  check(input, respond) { ... },
});

// RegExp matcher — pattern match
defineHook("PreToolUse", {
  matcher: /^(Write|Edit)$/,
  check(input, respond) { ... },
});

// No matcher — runs for all instances of the event
defineHook("Stop", {
  check(input, respond) { ... },
});
```

Matcher fields by event: `PreToolUse`/`PostToolUse`/`PostToolUseFailure`/`PermissionRequest` match on `tool_name`. `SessionStart` on `source`. `Notification` on `notification_type`. `SubagentStart`/`SubagentStop` on `agent_type`. See [config](../config/index.md) for the full mapping.

## Check function patterns

The `check` function receives typed input and response builders. It can return:
- A response object (synchronous)
- A Promise (async)
- An Effect (for Effect-based pipelines)

### Synchronous check

```typescript
defineHook("PreToolUse", {
  matcher: "Bash",
  check(input, respond) {
    const cmd = (input.tool_input as Record<string, string>).command ?? "";
    if (/dangerous/.test(cmd)) {
      return respond.deny("Nope");
    }
    return respond.allow();
  },
});
```

### Async check

```typescript
defineHook("PostToolUse", {
  matcher: "Write|Edit",
  check: async (input, respond) => {
    const filePath = (input.tool_input as Record<string, string>).file_path ?? "";
    const proc = Bun.spawn(["biome", "check", filePath]);
    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      return respond.block("Lint failed");
    }
    return respond.proceed();
  },
});
```

## Response builders by family

The `respond` object passed to `check` varies by event:

### PreToolUse responses

```typescript
respond.allow()                           // Allow the tool use
respond.deny("reason")                    // Block with reason (exit 2)
respond.ask("reason?")                    // Prompt user for permission
respond.allowWithInput({ command: "..." }, "context")  // Allow with modified input
```

### PostToolUse / UserPromptSubmit / ConfigChange responses

```typescript
respond.proceed()                         // Continue normally
respond.block("reason", "context?")       // Block with reason (exit 2)
```

### PermissionRequest responses

```typescript
respond.allow()                           // Grant permission
respond.deny()                            // Deny permission
respond.allowWithDecision({ message: "...", interrupt: true })
```

### Stop / SubagentStop / TeammateIdle / TaskCompleted responses

```typescript
respond.allowStop()                       // Let it stop
respond.preventStop("reason")             // Keep going (exit 2)
```

### WorktreeCreate responses

```typescript
respond.path("/absolute/path/to/worktree")  // Return worktree path
```

### SessionStart / SessionEnd / InstructionsLoaded / SubagentStart / Notification / WorktreeRemove / PreCompact responses

```typescript
respond.done()                            // Side effect complete
respond.context("additional context")     // Inject context into session
```

## Failure policies

Control what happens when the hook pipeline errors:

```typescript
defineHook("PreToolUse", {
  matcher: "Bash",
  failurePolicy: "fail-open",   // Default: allow on error (exit 0)
  check(input, respond) { ... },
});

defineHook("PreToolUse", {
  matcher: "Bash",
  failurePolicy: "fail-closed", // Block on error (exit 2)
  check(input, respond) { ... },
});
```

- **`fail-open`** (default) — If the hook crashes, the action proceeds. Logs a warning to stderr.
- **`fail-closed`** — If the hook crashes, the action is blocked. Use for security-critical guards.

## Accessing raw input

The decoded input includes a `_raw` property with the original untyped JSON, useful for accessing fields not in the schema:

```typescript
defineHook("PreToolUse", {
  check(input, respond) {
    const raw = input._raw; // Record<string, unknown>
    const customField = raw["my_custom_field"];
    return respond.allow();
  },
});
```
