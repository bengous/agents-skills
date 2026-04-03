# PreToolUse Event

Fires **before** a tool executes. The most commonly used hook event — used for guards, input modification, and approval flows.

## When it fires

Every time Claude is about to call a tool (Bash, Write, Edit, Read, Glob, Grep, etc.), a PreToolUse event fires with the tool name and input.

## Input fields

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | string | Name of the tool (e.g., "Bash", "Write", "Edit") |
| `tool_input` | Record<string, unknown> | Tool-specific input (e.g., `{ command: "..." }` for Bash, `{ file_path: "...", content: "..." }` for Write) |
| `tool_use_id` | string? | Unique identifier for this tool use |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

## Response family: PreToolUseResponse

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Allow | 0 | — | — | Tool executes normally |
| Deny | 2 | — | reason | Tool use is blocked; reason shown to Claude |
| Ask | 0 | `{"hookSpecificOutput":{"permissionDecision":"ask","additionalContext":"..."}}` | — | User is prompted for permission |
| AllowWithInput | 0 | `{"hookSpecificOutput":{"permissionDecision":"allow","updatedInput":{...},"additionalContext":"..."}}` | — | Tool executes with modified input |

## Matcher field

`tool_name` — In settings.json, use `"matcher": "Bash"` to run only for Bash tool uses, or `"matcher": "Write|Edit"` for multiple tools.

## Settings registration

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "type": "command",
        "command": "bash .claude/hooks/guard-destructive.sh",
        "matcher": "Bash"
      }
    ]
  }
}
```

## Use cases

### Guard dangerous commands
Block specific bash commands (rm -rf, git push --force, etc.) before they execute. This is the most common PreToolUse hook. See the [block-destructive-bash example](../../examples/block-destructive-bash/index.md).

### Protect sensitive files
Block writes or edits to protected paths (.env, credentials, lock files). Match on `Write|Edit` and check `tool_input.file_path`. See the [protect-paths example](../../examples/protect-paths/index.md).

### Require approval for specific tools
Use the Ask response to force user confirmation before certain tools run, without fully blocking them.

### Modify tool input
Use AllowWithInput to transparently transform tool inputs — for example, adding flags to commands or normalizing file paths.
