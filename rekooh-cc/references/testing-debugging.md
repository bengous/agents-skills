# Testing And Debugging

Every hook change should have two proofs: static config validation and at least
one direct payload smoke test.

## Validate Settings

```bash
bun <skill-root>/scripts/validate-hooks.ts --settings .claude/settings.json
```

This checks local JSON shape and command hook script paths. It warns about
advanced handler types instead of pretending to validate the full product
contract.

## Direct Smoke Test

Run command hooks with representative stdin:

```bash
printf '%s\n' '{
  "session_id": "test",
  "transcript_path": "/tmp/transcript.jsonl",
  "cwd": "'"$PWD"'",
  "hook_event_name": "PreToolUse",
  "permission_mode": "default",
  "tool_name": "Bash",
  "tool_input": { "command": "rm -rf dist" }
}' | .claude/hooks/guard-bash.sh
```

Expected blocking hooks should exit `2` and print a clear reason to stderr.

## Claude Code Debugging

Useful official surfaces:

- `/hooks` to inspect configured hooks
- `claude --debug` or `claude --debug-file <path>` for logs
- `CLAUDE_CODE_DEBUG_LOG_LEVEL=verbose` for matcher/debug detail
- `/doctor` and `/context` when config or context loading looks wrong

Check the current hooks reference before relying on debug flags in automation.

## Report Format

When auditing, report:

- settings files found
- events registered
- handler types present
- local command paths missing or non-executable
- personal vs project scope
- official docs consulted
- remaining risk if validation was static-only
