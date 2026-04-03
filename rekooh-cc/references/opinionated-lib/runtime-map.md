# Runtime Map — _hooks-lib Vendored Source

Module-by-module responsibility mapping for the vendored typed runtime.

## Core (`src/core/`)

| Module | Responsibility |
|--------|---------------|
| `define-hook.ts` | Main entry point — `defineHook()` reads stdin, decodes event via Effect pipeline, runs matcher short-circuit, invokes check callback, renders response, and exits with appropriate code. Supports matcher (string/RegExp), failurePolicy, and check callbacks returning sync values, Effects, or Promises. |
| `common-input.ts` | Effect Schema for fields present on every hook event's stdin JSON: `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, optional `agent_id`/`agent_type`. |
| `decode.ts` | JSON parsing (`parseJson`) and schema-based event decoding (`decodeEvent`). Selects the correct schema from the registry and attaches `_raw` for access to untyped fields. |
| `errors.ts` | Tagged error types for the pipeline: `StdinError`, `DecodeError`, `CheckError`. All extend Effect's `Data.TaggedError`. |
| `event-inputs.ts` | Effect Schemas for all 18 event input types. Each extends `CommonInput` with event-specific fields. Exports `EventSchemas` registry mapping event names to schemas, and the `InputForEvent` type-level map. |
| `events.ts` | Canonical list of all 18 `EventName` literals as a const array plus an Effect `Schema.Literal` type. Single source of truth for event names. |
| `exit-codes.ts` | `HookExit` enum: `Allow` (0), `Error` (1), `Block` (2). Maps hook outcomes to process exit codes. |
| `failure-policy.ts` | `FailurePolicy` type (`"fail-open"` / `"fail-closed"`) and `applyPolicy` — an Effect error handler that converts pipeline errors to either allow (fail-open) or block (fail-closed) outputs. |
| `index.ts` | Barrel re-export of all core modules. Defines the public API surface for the `core/` package. |
| `response-builders.ts` | Type-safe response builder namespaces (`preToolUse`, `permissionRequest`, `topLevel`, `continueStop`, `worktree`, `sideEffect`). Each provides factory functions like `.allow()`, `.deny()`, `.block()`. Also maps events to their builder namespace via `getResponders()`. |
| `response-families.ts` | Discriminated union types for the 6 response families: `PreToolUseResponse`, `PermissionRequestResponse`, `TopLevelDecisionResponse`, `ContinueStopResponse`, `WorktreeResponse`, `SideEffectResponse`. Each uses `_tag` discriminant. |
| `response-map.ts` | `ResponseForEvent` type mapping each of the 18 event names to its response family type. Includes compile-time exhaustiveness check. |
| `response-render.ts` | Renders typed response objects into Claude-compatible `HookOutput` (`exitCode`/`stdout`/`stderr`). One renderer per family, dispatched by `renderResponse()` using the event→family mapping. |
| `stdin.ts` | Synchronous stdin reader using `fs.readFileSync(0)` wrapped in an Effect. Returns the raw string for downstream JSON parsing. |

## Testing (`src/testing/`)

| Module | Responsibility |
|--------|---------------|
| `test-hook.ts` | Subprocess test runner — `testHook(path)` returns `{ run(input), runRaw(stdin) }` that spawns the hook as a Bun child process, feeds JSON stdin, and captures `exitCode`/`stdout`/`stderr` plus a `.json()` helper. |
| `factories.ts` | Test payload factories — `createPayload("PreToolUse", overrides)` produces a complete, valid event payload with sensible defaults for all 18 event types. |
| `index.ts` | Barrel re-export of `testHook` and `createPayload`. |

## Utilities (`src/lib/`)

| Module | Responsibility |
|--------|---------------|
| `strip-literals.ts` | `stripStringLiterals(cmd)` — removes heredocs, double-quoted strings, and single-quoted strings from shell commands. Used by guard presets to prevent false positives when destructive commands appear inside string literals. |
