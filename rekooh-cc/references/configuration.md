# Configuration

Claude Code hook config lives under a top-level `hooks` object. Each event maps
to an array of groups. Each group can have a `matcher` and contains one or more
handlers.

Always verify advanced fields against `https://code.claude.com/docs/en/hooks`.

## Command Hook Shape

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/guard-bash.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

Notes:

- `matcher` filters on the event's matcher field, such as `tool_name` for tool
  events.
- Empty or omitted matcher means match all for events that support matching.
- Timeout values are seconds in current Claude Code docs.
- Keep commands relative to the project root when committing project settings.

## Exec-Style Command Hook

Use `args` when you want clearer argument boundaries and less shell parsing.
Verify the exact current schema in the hooks reference before relying on this in
shared config.

```json
{
  "type": "command",
  "command": "bash",
  "args": [".claude/hooks/guard-bash.sh"],
  "timeout": 30
}
```

## Useful Common Fields

Current docs include fields such as:

- `type`
- `if`
- `timeout`
- `statusMessage`
- `command`
- `args`
- `async`
- `asyncRewake`
- `shell`

These fields are product-owned and may change. Prefer official docs over this
reference for exact syntax.

## Storage Rules

- Put shared project hooks in `.claude/settings.json`.
- Put personal or machine-specific hooks in `.claude/settings.local.json`.
- Do not commit absolute home paths unless the repo intentionally targets one
  machine.
- Keep hook scripts under `.claude/hooks/` unless the project already has a
  stronger convention.
- Settings should reference scripts, not embed long shell one-liners.

## Idempotent Registration

For command hooks:

```bash
bun <skill-root>/scripts/patch-settings.ts \
  --settings .claude/settings.json \
  --event PreToolUse \
  --matcher Bash \
  --command ".claude/hooks/guard-bash.sh" \
  --timeout 30
```

The script merges by event, matcher, command, and args. It does not manage HTTP,
MCP, prompt, or agent handlers.
