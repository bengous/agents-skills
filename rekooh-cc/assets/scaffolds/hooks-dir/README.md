# .claude/hooks/

Claude Code command hooks directory.

Place hook scripts here and reference them in `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [".claude/hooks/guard-destructive.sh"]
      }
    ]
  }
}
```

Each hook receives JSON on stdin and communicates via exit codes:
- **Exit 0** — Allow the tool call
- **Exit 1** — Error (fail-open by default)
- **Exit 2** — Block the tool call (stderr = reason shown to Claude)
