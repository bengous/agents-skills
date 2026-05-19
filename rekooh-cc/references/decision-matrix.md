# Hook Decision Matrix

Use hooks only when deterministic automation at a Claude Code lifecycle point is
the right tool. Many requests are better solved with scripts, docs, or agent
instructions.

## Should This Be A Hook?

| Situation | Verdict | Better default |
|---|---|---|
| Block a known dangerous command or path | Hook fits | `PreToolUse` command hook |
| Format or lint after edits | Hook fits | `PostToolUse` command hook |
| Notify when Claude needs attention | Hook fits | `Notification` command hook |
| Inject stable project context at session start | Hook fits | `SessionStart` context hook |
| Enforce a team-wide Claude Code policy | Hook may fit | Project settings, documented clearly |
| Run business logic or production deploys | Usually no | Explicit project command |
| Make a subjective judgment | Usually no | Prompt/agent hook only after docs check |
| Fix code after every edit with broad side effects | Usually no | Explicit validation command |
| Generic Git lifecycle automation | No | Git hooks, not Claude Code hooks |

## Handler Choice

| Handler | Use When | Avoid When |
|---|---|---|
| `command` | Local deterministic script, easiest to test and review | Needs remote service state |
| `http` | Existing internal service should evaluate events | Local script is enough |
| `mcp_tool` | Existing MCP tool already owns the behavior | Hook would just proxy shell logic |
| `prompt` | You need model judgment at a lifecycle point | Deterministic rule is possible |
| `agent` | A larger agent workflow should react to the event | A small script can decide |

This skill's scripts manage command hooks only. For other handler types, consult
the official hooks reference and edit settings directly.

## Scope Choice

| Scope | Use When |
|---|---|
| Project `.claude/settings.json` | Team/project policy should be committed |
| Local `.claude/settings.local.json` | Personal workflow or machine-specific command |
| User `~/.claude/settings.json` | Same behavior across many repos |
| Managed policy | Organization-administered enforcement |
| Plugin or skill frontmatter | Packaged extension owns the hook |

## Failure Policy

| Hook Kind | Default |
|---|---|
| Destructive-command guard | Fail closed only after direct smoke tests |
| Formatter/linter after edit | Fail open with visible diagnostics |
| Notification | Fail open |
| Context injection | Fail open unless missing context is dangerous |
| Worktree lifecycle override | Be strict; bad paths can break isolation |

## Stop Conditions

Stop and re-check official docs when:

- The event or handler type is not in the local references.
- You need `http`, `mcp_tool`, `prompt`, `agent`, async behavior, or `if`.
- You are changing managed/user-level settings.
- A hook blocks unexpectedly in a live Claude Code session.
