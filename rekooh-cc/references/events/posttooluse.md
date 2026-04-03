# PostToolUse and PostToolUseFailure Events

Fire **after** a tool executes — PostToolUse on success, PostToolUseFailure on failure.

## PostToolUse — When it fires

After a tool completes successfully. Use for post-execution validation, auto-formatting, logging.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | string | Name of the tool that executed |
| `tool_input` | Record<string, unknown> | The input that was passed to the tool |
| `tool_use_id` | string? | Unique identifier for this tool use |
| `tool_response` | Record<string, unknown>? | The tool's output/response |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

## PostToolUseFailure — When it fires

After a tool execution fails. Use for error logging, retry logic, or blocking on specific failure patterns.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | string | Name of the tool that failed |
| `tool_input` | Record<string, unknown> | The input that was passed to the tool |
| `tool_use_id` | string? | Unique identifier for this tool use |
| `error` | string | Error message from the failed execution |
| `is_interrupt` | boolean? | Whether the failure was due to an interrupt |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

## Response family: TopLevelDecisionResponse

Both events use the same response family:

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Proceed | 0 | — | — | Continue normally |
| Block | 2 | — | reason | Block further processing; reason shown to Claude |

## Matcher field

`tool_name` — Same as PreToolUse. Use `"matcher": "Write|Edit"` to run only after file modifications.

## Settings registration

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "type": "command",
        "command": "bash .claude/hooks/lint-after-edit.sh",
        "matcher": "Write|Edit"
      }
    ],
    "PostToolUseFailure": [
      {
        "type": "command",
        "command": "bash .claude/hooks/log-failures.sh",
        "matcher": "Bash"
      }
    ]
  }
}
```

## Use cases

### Auto-lint after file edits (PostToolUse)
Run a linter or formatter after every Write/Edit operation. Block if the linted file has errors. See the [run-lint-after-edit example](../../examples/run-lint-after-edit/index.md).

### Log tool usage (PostToolUse)
Append tool name, input, and timestamp to a log file for audit trails.

### Monitor failures (PostToolUseFailure)
Track failure patterns — if the same command keeps failing, surface a warning or block further attempts.
