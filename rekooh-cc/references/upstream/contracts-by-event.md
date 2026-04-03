# Contracts by Event — Cheat Sheet

Compact reference for all 18 hook events derived from the typed runtime source code.

## Event contracts

| Event | Input fields (beyond common) | Response family | Matcher field | Exit: allow | Exit: block |
|-------|------------------------------|----------------|---------------|-------------|-------------|
| **SessionStart** | `source`, `model?` | SideEffectResponse | `source` | 0 | — |
| **SessionEnd** | `reason` | SideEffectResponse | `reason` | 0 | — |
| **InstructionsLoaded** | `file_path`, `memory_type`, `load_reason`, `globs?`, `trigger_file_path?`, `parent_file_path?` | SideEffectResponse | — | 0 | — |
| **UserPromptSubmit** | `prompt` | TopLevelDecisionResponse | — | 0 | 2 |
| **PreToolUse** | `tool_name`, `tool_input`, `tool_use_id?` | PreToolUseResponse | `tool_name` | 0 | 2 |
| **PermissionRequest** | `tool_name`, `tool_input`, `permission_suggestions?` | PermissionRequestResponse | `tool_name` | 0 | — |
| **PostToolUse** | `tool_name`, `tool_input`, `tool_use_id?`, `tool_response?` | TopLevelDecisionResponse | `tool_name` | 0 | 2 |
| **PostToolUseFailure** | `tool_name`, `tool_input`, `tool_use_id?`, `error`, `is_interrupt?` | TopLevelDecisionResponse | `tool_name` | 0 | 2 |
| **Notification** | `message`, `title?`, `notification_type` | SideEffectResponse | `notification_type` | 0 | — |
| **PreCompact** | `trigger`, `custom_instructions?` | SideEffectResponse | `trigger` | 0 | — |
| **SubagentStart** | `agent_id`, `agent_type` | SideEffectResponse | `agent_type` | 0 | — |
| **SubagentStop** | `stop_hook_active`, `agent_id`, `agent_type`, `agent_transcript_path`, `last_assistant_message?` | ContinueStopResponse | `agent_type` | 0 | 2 |
| **Stop** | `stop_hook_active`, `last_assistant_message?` | ContinueStopResponse | — | 0 | 2 |
| **TeammateIdle** | `teammate_name`, `team_name` | ContinueStopResponse | — | 0 | 2 |
| **TaskCompleted** | `task_id`, `task_subject`, `task_description?`, `teammate_name?`, `team_name?` | ContinueStopResponse | — | 0 | 2 |
| **ConfigChange** | `source`, `file_path?` | TopLevelDecisionResponse | `source` | 0 | 2 |
| **WorktreeCreate** | `name` | WorktreeResponse | — | 0 | — |
| **WorktreeRemove** | `worktree_path` | SideEffectResponse | — | 0 | — |

## Common input fields (all events)

| Field | Type |
|-------|------|
| `session_id` | string |
| `transcript_path` | string |
| `cwd` | string |
| `permission_mode` | string |
| `hook_event_name` | string |
| `agent_id` | string? |
| `agent_type` | string? |

## Response families

### PreToolUseResponse
| Tag | Exit | Stdout | Stderr |
|-----|------|--------|--------|
| Allow | 0 | — | — |
| Deny | 2 | — | reason |
| Ask | 0 | `{"hookSpecificOutput":{"permissionDecision":"ask","additionalContext":"..."}}` | — |
| AllowWithInput | 0 | `{"hookSpecificOutput":{"permissionDecision":"allow","updatedInput":{...},"additionalContext":"..."}}` | — |

### PermissionRequestResponse
| Tag | Exit | Stdout | Stderr |
|-----|------|--------|--------|
| Allow | 0 | `{"hookSpecificOutput":{"decision":{"behavior":"allow"}}}` | — |
| Deny | 0 | `{"hookSpecificOutput":{"decision":{"behavior":"deny"}}}` | — |
| AllowWithDecision | 0 | `{"hookSpecificOutput":{"decision":{"behavior":"allow","updatedInput":...,"message":...}}}` | — |

### TopLevelDecisionResponse
| Tag | Exit | Stdout | Stderr |
|-----|------|--------|--------|
| Proceed | 0 | — | — |
| Block | 2 | — | reason |

### ContinueStopResponse
| Tag | Exit | Stdout | Stderr |
|-----|------|--------|--------|
| AllowStop | 0 | — | — |
| PreventStop | 2 | — | reason |

### WorktreeResponse
| Tag | Exit | Stdout | Stderr |
|-----|------|--------|--------|
| Path | 0 | absolute path (plain string) | — |

### SideEffectResponse
| Tag | Exit | Stdout | Stderr |
|-----|------|--------|--------|
| Done | 0 | — | — |
| Context | 0 | `{"hookSpecificOutput":{"additionalContext":"..."}}` | — |

## Exit code semantics

| Code | Meaning |
|------|---------|
| 0 | Allow — hook approves the action |
| 1 | Error — hook failed; action proceeds, warning logged |
| 2 | Block — hook explicitly blocks; stderr has the reason |
