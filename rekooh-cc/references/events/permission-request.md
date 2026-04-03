# PermissionRequest Event

Fires when a tool requests permission from the user. Allows hooks to auto-approve, auto-deny, or modify the permission decision.

## When it fires

When Claude's permission system would normally prompt the user for approval (e.g., before executing a command that hasn't been pre-approved). This fires after PreToolUse but before the permission dialog.

## Input fields

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | string | Name of the tool requesting permission |
| `tool_input` | Record<string, unknown> | The tool's input |
| `permission_suggestions` | unknown? | Suggested permission settings |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

## Response family: PermissionRequestResponse

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Allow | 0 | `{"hookSpecificOutput":{"decision":{"behavior":"allow"}}}` | — | Permission granted automatically |
| Deny | 0 | `{"hookSpecificOutput":{"decision":{"behavior":"deny"}}}` | — | Permission denied automatically |
| AllowWithDecision | 0 | `{"hookSpecificOutput":{"decision":{"behavior":"allow",...}}}` | — | Permission granted with modifications |

Note: All PermissionRequest responses use exit code 0. The decision is communicated through the stdout JSON, not the exit code.

### AllowWithDecision options

```json
{
  "hookSpecificOutput": {
    "decision": {
      "behavior": "allow",
      "updatedInput": { "command": "modified command" },
      "updatedPermissions": [],
      "message": "Auto-approved by hook",
      "interrupt": false
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `updatedInput` | Record<string, unknown>? | Modified tool input |
| `updatedPermissions` | unknown[]? | Modified permission settings |
| `message` | string? | Message to display about the decision |
| `interrupt` | boolean? | Whether to interrupt and show the message |

## Matcher field

`tool_name` — Use `"matcher": "Bash"` to handle only Bash permission requests.

## Settings registration

```json
{
  "hooks": {
    "PermissionRequest": [
      {
        "type": "command",
        "command": "bash .claude/hooks/auto-approve.sh",
        "matcher": "Bash"
      }
    ]
  }
}
```

## Use cases

### Auto-approve safe commands
Automatically grant permission for commands that match a known-safe pattern, reducing permission prompts.

### Auto-deny in restricted contexts
Automatically deny permission for tools in certain contexts (e.g., when working on production branches).
