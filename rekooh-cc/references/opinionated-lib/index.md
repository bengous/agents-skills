# Opinionated Typed Runtime

A vendored TypeScript runtime for authoring Claude Code hooks with full type safety.

## Overview

The opinionated typed runtime provides:
- **Effect-based pipeline** — stdin reading, JSON parsing, schema decoding, response rendering
- **18 typed event schemas** — compile-time validation of event input fields
- **6 response families** — type-safe builders that prevent invalid responses
- **Matcher support** — string or regex filtering on event-specific fields
- **Failure policies** — configurable fail-open or fail-closed behavior
- **Test utilities** — subprocess runner and payload factories

## Files in this directory

| File | Description |
|------|-------------|
| [create-hook-guide.md](create-hook-guide.md) | Step-by-step guide to creating a hook with `defineHook` |
| [runtime-map.md](runtime-map.md) | Module-by-module responsibility map of the vendored source |
| `source/` | Complete vendored source code for reading and reasoning |

## When to use the typed runtime

Use it when:
- You're building multiple hooks that share validation logic
- You want compile-time guarantees on event input shapes and response types
- You need test harness + payload factories
- The project already uses Bun and TypeScript

Don't use it when:
- A single standalone hook solves the problem
- The project doesn't use TypeScript
- You want zero dependencies

See the [strategy guide](../strategy/index.md) for a full comparison.

## Key concepts

### `defineHook(event, config)`

The single entry point. When the module executes, it:
1. Reads stdin
2. Parses JSON
3. Checks matcher (short-circuits with allow if no match)
4. Decodes input using the event's Effect schema
5. Calls your `check` function with typed input and response builders
6. Renders the response to stdout/stderr
7. Exits with the appropriate code (0, 1, or 2)

### Response builders

Each event family gets its own builder namespace. The `check` callback receives the correct builders automatically:

- `preToolUse` — `.allow()`, `.deny(reason)`, `.ask(reason?)`, `.allowWithInput(input, context?)`
- `permissionRequest` — `.allow()`, `.deny()`, `.allowWithDecision(opts)`
- `topLevel` — `.proceed()`, `.block(reason, context?)`
- `continueStop` — `.allowStop()`, `.preventStop(reason)`
- `worktree` — `.path(absolutePath)`
- `sideEffect` — `.done()`, `.context(text)`

### Failure policies

- `"fail-open"` (default) — On error, allow the action and log a warning
- `"fail-closed"` — On error, block the action

## Getting started

See [create-hook-guide.md](create-hook-guide.md) for a step-by-step walkthrough, or the [typed-hook-with-opinionated-lib example](../../examples/typed-hook-with-opinionated-lib/index.md) for a complete working example.
