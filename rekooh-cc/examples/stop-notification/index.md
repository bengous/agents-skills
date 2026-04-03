# Example: Desktop Notification on Stop

## Problem

Get a desktop notification when Claude finishes its work so you know to return to the terminal without constantly watching it.

## Solution

A Stop event hook that sends a desktop notification using the system's notification mechanism.

### `.claude/hooks/notify-stop.sh`

```bash
#!/usr/bin/env bash

INPUT=$(cat)
LAST_MESSAGE=$(echo "$INPUT" | jq -r '.last_assistant_message // "Claude has stopped"' | head -c 200)

# Detect platform and send notification
if command -v notify-send &>/dev/null; then
  # Linux (libnotify)
  notify-send "Claude Code" "$LAST_MESSAGE" 2>/dev/null || true
elif command -v osascript &>/dev/null; then
  # macOS
  osascript -e "display notification \"$LAST_MESSAGE\" with title \"Claude Code\"" 2>/dev/null || true
elif command -v powershell.exe &>/dev/null; then
  # WSL
  powershell.exe -Command "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); [System.Windows.Forms.MessageBox]::Show('$LAST_MESSAGE', 'Claude Code')" 2>/dev/null || true
fi

# Always allow stop — notifications should never block
exit 0
```

Make it executable:

```bash
chmod +x .claude/hooks/notify-stop.sh
```

## Settings registration

Add to `.claude/settings.json`:

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

No matcher is needed — the Stop event doesn't support matchers.

## Validation

Test the notification:

```bash
echo '{"stop_hook_active":false,"last_assistant_message":"I have finished implementing the feature.","session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"Stop"}' | bash .claude/hooks/notify-stop.sh
echo "Exit code: $?"
# Expected: Exit code: 0 (always allows stop)
# Expected: Desktop notification appears
```

Test without a last message:

```bash
echo '{"stop_hook_active":false,"session_id":"test","transcript_path":"/tmp/t","cwd":"/tmp","permission_mode":"default","hook_event_name":"Stop"}' | bash .claude/hooks/notify-stop.sh
echo "Exit code: $?"
# Expected: Exit code: 0
# Expected: Notification with "Claude has stopped" fallback message
```

## Variations

### Sound alert instead of notification

```bash
#!/usr/bin/env bash
INPUT=$(cat)
# Play system bell
printf '\a' 2>/dev/null || true
# Or play a sound file
paplay /usr/share/sounds/freedesktop/stereo/complete.oga 2>/dev/null || true
exit 0
```

### Log to file instead of notification

```bash
#!/usr/bin/env bash
INPUT=$(cat)
LAST_MESSAGE=$(echo "$INPUT" | jq -r '.last_assistant_message // "stopped"' | head -c 500)
echo "[$(date -Iseconds)] Claude stopped: $LAST_MESSAGE" >> "$HOME/.claude/stop-log.txt"
exit 0
```
