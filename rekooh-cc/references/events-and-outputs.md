# Events And Outputs

This file is a compact working reference. Check
`https://code.claude.com/docs/en/hooks` for exact current schemas before
implementing unfamiliar events.

## Current Event Families To Check

The official docs currently include events around:

- Session: `SessionStart`, `Setup`, `SessionEnd`
- Prompt: `UserPromptSubmit`, `UserPromptExpansion`
- Tools: `PreToolUse`, `PermissionRequest`, `PermissionDenied`,
  `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`
- Notifications: `Notification`
- Agents and tasks: `SubagentStart`, `SubagentStop`, `TaskCreated`,
  `TaskCompleted`, `TeammateIdle`
- Stop flow: `Stop`, `StopFailure`
- Context and config: `InstructionsLoaded`, `ConfigChange`, `PreCompact`,
  `PostCompact`
- Files and cwd: `CwdChanged`, `FileChanged`
- Worktrees: `WorktreeCreate`, `WorktreeRemove`
- Elicitation: `Elicitation`, `ElicitationResult`

Treat this as a checklist, not a frozen schema.

## Common Input

Most hook inputs include:

- `session_id`
- `transcript_path`
- `cwd`
- `hook_event_name`
- often `permission_mode`
- subagent fields such as `agent_id` and `agent_type` when relevant

Tool events include `tool_name` and `tool_input`.

## PreToolUse Decisions

Allow silently:

```bash
exit 0
```

Deny with a reason:

```bash
echo "Blocked by project hook: explain the exact rule" >&2
exit 2
```

Ask or modify input with JSON on stdout:

```json
{
  "hookSpecificOutput": {
    "permissionDecision": "ask",
    "permissionDecisionReason": "This command touches deployment config."
  }
}
```

```json
{
  "hookSpecificOutput": {
    "permissionDecision": "allow",
    "updatedInput": {
      "command": "bun run test"
    },
    "additionalContext": "Replaced broad validation with the project test command."
  }
}
```

## General Output Fields

Current docs include general output controls such as:

- `continue`
- `stopReason`
- `suppressOutput`
- `systemMessage`
- `terminalSequence`
- event-specific `hookSpecificOutput`

Do not assume every event honors every field. Verify per event.

## Exit Codes

For command hooks, the practical baseline is:

- `0`: success, allow/continue unless stdout says otherwise
- `2`: intentional block for events that support blocking
- other non-zero: hook error; behavior is event-specific

Worktree and lifecycle events can have stricter behavior. Check official docs
before using hooks that replace core Claude Code behavior.
