# Config — Settings.json Hook Registration

Register hooks in `.claude/settings.json` so Claude Code executes them at the right events.

## Structure

The `hooks` object in settings.json maps event names to arrays of hook groups:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/guard-destructive.sh",
            "timeout": 10000
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "bun .claude/hooks/lint-after-edit.ts"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/notify-stop.sh"
          }
        ]
      }
    ]
  }
}
```

## Hook configuration fields

| Field | Required | Description |
|-------|----------|-------------|
| `type` | Yes | Always `"command"` |
| `command` | Yes | Shell command to execute. Receives event JSON on stdin. |
| `matcher` | No | Regex pattern tested against the event's matcher field (e.g., `tool_name` for PreToolUse). If omitted, the hook runs for all instances of the event. |
| `timeout` | No | Maximum execution time in milliseconds. Defaults to Claude Code's built-in timeout. |

## Matcher field by event

Not all events support matchers. The matcher field varies by event:

| Event | Matcher field | Example matcher |
|-------|---------------|-----------------|
| PreToolUse | `tool_name` | `"Bash"`, `"Write\|Edit"` |
| PostToolUse | `tool_name` | `"Write\|Edit"` |
| PostToolUseFailure | `tool_name` | `"Bash"` |
| PermissionRequest | `tool_name` | `"Bash"` |
| SessionStart | `source` | `"startup"` |
| SessionEnd | `reason` | `"prompt_input_exit"` |
| Notification | `notification_type` | `"permission_prompt"` |
| SubagentStart | `agent_type` | `"Explore"` |
| SubagentStop | `agent_type` | `"Explore"` |
| PreCompact | `trigger` | `"manual"` |
| ConfigChange | `source` | `"project_settings"` |

Events not listed (Stop, TeammateIdle, TaskCompleted, WorktreeCreate, WorktreeRemove, UserPromptSubmit, InstructionsLoaded) do not support matchers.

## Multiple hooks per event

Multiple hooks on the same event run in parallel. When hooks disagree:
- The most restrictive decision wins (block > ask > allow)
- All hooks' stderr output is collected

## Settings file locations

| File | Scope | Committed to git? |
|------|-------|--------------------|
| `.claude/settings.json` | Project-wide | Yes |
| `.claude/settings.local.json` | User-local overrides | No (gitignored) |

## Patching settings programmatically

Use the patch-settings script to add or update hook registrations without manual editing:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/patch-settings.ts
```

The script inserts hooks idempotently — it won't create duplicate registrations for the same event + matcher + command combination.

## Examples

Register a PreToolUse guard for Bash commands:
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/guard-destructive.sh"
          }
        ]
      }
    ]
  }
}
```

Register a PostToolUse linter for file edits:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/lint-after-edit.sh"
          }
        ]
      }
    ]
  }
}
```

Register a Stop notification (no matcher needed):
```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/notify-stop.sh"
          }
        ]
      }
    ]
  }
}
```
