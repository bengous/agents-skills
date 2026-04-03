# User and Config Events — UserPromptSubmit and ConfigChange

Events related to user input and configuration changes.

## UserPromptSubmit

### When it fires
When the user submits a prompt to Claude.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `prompt` | string | The user's submitted prompt text |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Response family: TopLevelDecisionResponse

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Proceed | 0 | — | — | Prompt is processed normally |
| Block | 2 | — | reason | Prompt is rejected; reason shown to user |

### Matcher field
None — UserPromptSubmit does not support matchers.

### Settings registration

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "type": "command",
        "command": "bash .claude/hooks/validate-prompt.sh"
      }
    ]
  }
}
```

### Use cases
- Validate prompts before processing (e.g., block prompts containing sensitive data)
- Transform or augment prompts before they reach Claude
- Log all user prompts for audit

---

## ConfigChange

### When it fires
When a settings or configuration file changes during a session.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `source` | string | What triggered the config change (e.g., "project_settings") |
| `file_path` | string? | Path to the changed configuration file |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Response family: TopLevelDecisionResponse

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Proceed | 0 | — | — | Config change is applied |
| Block | 2 | — | reason | Config change is rejected |

### Matcher field
`source` — Use `"matcher": "project_settings"` to handle only project settings changes.

### Settings registration

```json
{
  "hooks": {
    "ConfigChange": [
      {
        "type": "command",
        "command": "bash .claude/hooks/validate-config.sh",
        "matcher": "project_settings"
      }
    ]
  }
}
```

### Use cases
- Validate settings changes before they take effect
- Block unauthorized modifications to hook configurations
- Log configuration changes for audit trails
