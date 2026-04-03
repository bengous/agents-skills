# Events — Hook Event Reference

Claude Code fires 18 hook events at different points in its lifecycle. Each event delivers specific input fields and expects responses from a specific response family.

## Events by family

### Tool events
| Event | When it fires | Response family | Reference |
|-------|---------------|-----------------|-----------|
| PreToolUse | Before a tool executes | PreToolUseResponse | [pretooluse.md](pretooluse.md) |
| PostToolUse | After a tool succeeds | TopLevelDecisionResponse | [posttooluse.md](posttooluse.md) |
| PostToolUseFailure | After a tool fails | TopLevelDecisionResponse | [posttooluse.md](posttooluse.md) |
| PermissionRequest | When tool requests permission | PermissionRequestResponse | [permission-request.md](permission-request.md) |

### User interaction events
| Event | When it fires | Response family | Reference |
|-------|---------------|-----------------|-----------|
| UserPromptSubmit | User submits a prompt | TopLevelDecisionResponse | [user-config.md](user-config.md) |
| ConfigChange | Settings file changes | TopLevelDecisionResponse | [user-config.md](user-config.md) |

### Session lifecycle events
| Event | When it fires | Response family | Reference |
|-------|---------------|-----------------|-----------|
| SessionStart | Session begins | SideEffectResponse | [lifecycle.md](lifecycle.md) |
| SessionEnd | Session ends | SideEffectResponse | [lifecycle.md](lifecycle.md) |
| InstructionsLoaded | CLAUDE.md file loaded | SideEffectResponse | [lifecycle.md](lifecycle.md) |

### Notification events
| Event | When it fires | Response family | Reference |
|-------|---------------|-----------------|-----------|
| Notification | System notification | SideEffectResponse | [notification.md](notification.md) |
| PreCompact | Before context compaction | SideEffectResponse | [notification.md](notification.md) |

### Agent events
| Event | When it fires | Response family | Reference |
|-------|---------------|-----------------|-----------|
| SubagentStart | Subagent launches | SideEffectResponse | [subagent.md](subagent.md) |
| SubagentStop | Subagent finishes | ContinueStopResponse | [subagent.md](subagent.md) |
| TeammateIdle | Teammate becomes idle | ContinueStopResponse | [subagent.md](subagent.md) |
| TaskCompleted | Task marked complete | ContinueStopResponse | [subagent.md](subagent.md) |

### Stop event
| Event | When it fires | Response family | Reference |
|-------|---------------|-----------------|-----------|
| Stop | Claude is about to stop | ContinueStopResponse | [stop.md](stop.md) |

### Worktree events
| Event | When it fires | Response family | Reference |
|-------|---------------|-----------------|-----------|
| WorktreeCreate | Worktree creation requested | WorktreeResponse | [worktree.md](worktree.md) |
| WorktreeRemove | Worktree removal requested | SideEffectResponse | [worktree.md](worktree.md) |

## Common input fields

Every event includes these base fields:

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Current session identifier |
| `transcript_path` | string | Path to the session transcript file |
| `cwd` | string | Current working directory |
| `permission_mode` | string | Active permission mode |
| `hook_event_name` | string | Name of the event (e.g., "PreToolUse") |
| `agent_id` | string? | Agent ID if running in a subagent |
| `agent_type` | string? | Agent type if running in a subagent |

## Response families summary

| Family | Options | Used by |
|--------|---------|---------|
| PreToolUseResponse | Allow, Deny, Ask, AllowWithInput | PreToolUse |
| PermissionRequestResponse | Allow, Deny, AllowWithDecision | PermissionRequest |
| TopLevelDecisionResponse | Proceed, Block | PostToolUse, PostToolUseFailure, UserPromptSubmit, ConfigChange |
| ContinueStopResponse | AllowStop, PreventStop | Stop, SubagentStop, TeammateIdle, TaskCompleted |
| WorktreeResponse | Path | WorktreeCreate |
| SideEffectResponse | Done, Context | SessionStart, SessionEnd, InstructionsLoaded, SubagentStart, Notification, WorktreeRemove, PreCompact |

## Quick reference

For a compact table of all 18 events with their fields, response families, and matcher fields, see [contracts-by-event](../upstream/contracts-by-event.md).
