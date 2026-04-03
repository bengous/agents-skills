# Subagent and Team Events

Events related to subagent lifecycle and team coordination.

## SubagentStart

### When it fires
When a subagent (Explore, Plan, general-purpose, etc.) is launched.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `agent_id` | string | Unique identifier of the subagent |
| `agent_type` | string | Type of subagent (e.g., "Explore", "Plan", "general-purpose") |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
`agent_type` — Use `"matcher": "Explore"` to run only when Explore agents launch.

### Response family: SideEffectResponse

| Option | Exit code | Effect |
|--------|-----------|--------|
| Done | 0 | No action |
| Context | 0 | Inject context into the subagent |

### Use cases
- Log subagent launches for monitoring
- Inject context specific to the subagent type

---

## SubagentStop

### When it fires
When a subagent finishes execution.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `stop_hook_active` | boolean | Whether a stop hook is active |
| `agent_id` | string | Unique identifier of the subagent |
| `agent_type` | string | Type of subagent |
| `agent_transcript_path` | string | Path to the subagent's transcript |
| `last_assistant_message` | string? | Last message from the subagent |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
`agent_type` — Use `"matcher": "Explore"` to handle only Explore agent completions.

### Response family: ContinueStopResponse

| Option | Exit code | Effect |
|--------|-----------|--------|
| AllowStop | 0 | Subagent stops normally |
| PreventStop | 2 | Keep the subagent running; reason sent via stderr |

### Use cases
- Review subagent output before allowing it to stop
- Prevent subagents from stopping before completing specific criteria

---

## TeammateIdle

### When it fires
When a teammate in a team becomes idle (finishes its current work).

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `teammate_name` | string | Name of the idle teammate |
| `team_name` | string | Name of the team |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
None — TeammateIdle does not support matchers.

### Response family: ContinueStopResponse

| Option | Exit code | Effect |
|--------|-----------|--------|
| AllowStop | 0 | Teammate remains idle |
| PreventStop | 2 | Assign more work to the teammate; reason sent via stderr |

### Use cases
- Auto-assign new tasks when teammates become idle
- Log team utilization

---

## TaskCompleted

### When it fires
When a task is marked as completed.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | string | ID of the completed task |
| `task_subject` | string | Subject/title of the task |
| `task_description` | string? | Full task description |
| `teammate_name` | string? | Name of the teammate that completed it |
| `team_name` | string? | Name of the team |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
None — TaskCompleted does not support matchers.

### Response family: ContinueStopResponse

| Option | Exit code | Effect |
|--------|-----------|--------|
| AllowStop | 0 | Normal completion |
| PreventStop | 2 | Reject the completion; reason sent via stderr |

### Use cases
- Validate task output before accepting completion
- Trigger downstream tasks when dependencies complete
- Log task completion metrics

---

## Settings registration examples

```json
{
  "hooks": {
    "SubagentStart": [
      {
        "type": "command",
        "command": "bash .claude/hooks/log-subagent.sh"
      }
    ],
    "SubagentStop": [
      {
        "type": "command",
        "command": "bash .claude/hooks/review-subagent.sh",
        "matcher": "Explore"
      }
    ]
  }
}
```
