# Lifecycle Events — SessionStart, SessionEnd, InstructionsLoaded

Events that fire during session lifecycle transitions.

## SessionStart

### When it fires
At the beginning of a new Claude Code session.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `source` | string | How the session was started (e.g., "startup") |
| `model` | string? | The model being used |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
`source` — Use `"matcher": "startup"` to run only on initial startup.

### Settings registration

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "bash .claude/hooks/session-setup.sh"
      }
    ]
  }
}
```

### Use cases
- Initialize temporary directories or state files
- Log session start for audit
- Inject initial context into the session via the Context response

---

## SessionEnd

### When it fires
When a session is ending.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `reason` | string | Why the session ended (e.g., "prompt_input_exit") |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
`reason` — Use `"matcher": "prompt_input_exit"` to run only on user-initiated exits.

### Settings registration

```json
{
  "hooks": {
    "SessionEnd": [
      {
        "type": "command",
        "command": "bash .claude/hooks/session-cleanup.sh"
      }
    ]
  }
}
```

### Use cases
- Clean up temporary files created during the session
- Log session duration and summary
- Send cleanup notifications

---

## InstructionsLoaded

### When it fires
When a CLAUDE.md or similar instructions file is loaded into the session context.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | string | Path to the loaded instructions file |
| `memory_type` | string | Type of memory (e.g., "Project") |
| `load_reason` | string | Why the file was loaded (e.g., "session_start") |
| `globs` | string? | Glob patterns associated with the file |
| `trigger_file_path` | string? | File that triggered the load |
| `parent_file_path` | string? | Parent instructions file |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Matcher field
None — InstructionsLoaded does not support matchers.

### Settings registration

```json
{
  "hooks": {
    "InstructionsLoaded": [
      {
        "type": "command",
        "command": "bash .claude/hooks/on-instructions-loaded.sh"
      }
    ]
  }
}
```

### Use cases
- Inject dynamic context based on which instructions file was loaded
- Log which CLAUDE.md files are active
- Validate instructions files for consistency

---

## Response family: SideEffectResponse

All three lifecycle events use SideEffectResponse:

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Done | 0 | — | — | No additional action |
| Context | 0 | `{"hookSpecificOutput":{"additionalContext":"..."}}` | — | Injects context into the session |

The Context response is particularly useful for SessionStart (inject initial instructions) and InstructionsLoaded (supplement loaded instructions).
