# Notification and PreCompact Events

Events for system notifications and context compaction.

## Notification

### When it fires
When Claude Code generates a system notification (e.g., permission prompts, status updates).

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `message` | string | Notification message content |
| `title` | string? | Notification title |
| `notification_type` | string | Type of notification (e.g., "permission_prompt") |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
`notification_type` — Use `"matcher": "permission_prompt"` to handle only permission-related notifications.

### Settings registration

```json
{
  "hooks": {
    "Notification": [
      {
        "type": "command",
        "command": "bash .claude/hooks/desktop-notify.sh"
      }
    ]
  }
}
```

### Use cases
- Forward notifications to desktop notification systems (notify-send, osascript)
- Log notifications for audit trails
- Filter or suppress specific notification types

---

## PreCompact

### When it fires
Before Claude Code compacts the conversation context (when the context window is getting full).

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `trigger` | string | What triggered the compaction (e.g., "manual", "auto") |
| `custom_instructions` | string? | Custom instructions for the compaction |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
`trigger` — Use `"matcher": "manual"` to run only on manually triggered compactions.

### Settings registration

```json
{
  "hooks": {
    "PreCompact": [
      {
        "type": "command",
        "command": "bash .claude/hooks/pre-compact.sh"
      }
    ]
  }
}
```

### Use cases
- Inject custom compaction instructions via the Context response to preserve important information
- Log when compaction occurs and what triggered it
- Save context snapshots before compaction

---

## Response family: SideEffectResponse

Both events use SideEffectResponse:

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Done | 0 | — | — | No additional action |
| Context | 0 | `{"hookSpecificOutput":{"additionalContext":"..."}}` | — | Injects context |

For PreCompact, the Context response is particularly useful — it can inject instructions that guide how the compaction summarizes the conversation.
