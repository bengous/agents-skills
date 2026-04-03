# Stop Event

Fires when Claude is about to stop its current turn. Allows hooks to prevent premature stops.

## When it fires

When Claude decides to stop responding — either because it completed its task, hit a stopping condition, or the user requested a stop.

## Input fields

| Field | Type | Description |
|-------|------|-------------|
| `stop_hook_active` | boolean | Whether a stop hook is already registered and active |
| `last_assistant_message` | string? | The last message Claude was about to send |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

## Response family: ContinueStopResponse

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| AllowStop | 0 | — | — | Claude stops normally |
| PreventStop | 2 | — | reason | Claude continues; reason injected as context |

## Matcher field

None — Stop does not support matchers.

## Settings registration

```json
{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "bash .claude/hooks/notify-stop.sh"
      }
    ]
  }
}
```

## Use cases

### Desktop notification on stop
Send a notification when Claude finishes so you know to return. See the [stop-notification example](../../examples/stop-notification/index.md).

```bash
#!/usr/bin/env bash
INPUT=$(cat)
MESSAGE=$(echo "$INPUT" | jq -r '.last_assistant_message // "Claude has stopped"' | head -c 200)

# Linux
notify-send "Claude stopped" "$MESSAGE" 2>/dev/null

# macOS (alternative)
# osascript -e "display notification \"$MESSAGE\" with title \"Claude stopped\""

exit 0
```

### Prevent stop before checklist
Prevent Claude from stopping before completing all items in a checklist:

```bash
#!/usr/bin/env bash
# Check if all TODO items are resolved
if grep -rq "TODO" .claude/checklist.md 2>/dev/null; then
  echo "Outstanding TODOs remain in checklist" >&2
  exit 2
fi
exit 0
```
