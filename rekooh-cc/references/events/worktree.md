# Worktree Events — WorktreeCreate and WorktreeRemove

Events for managing git worktree lifecycle.

## WorktreeCreate

### When it fires
When Claude requests creation of a git worktree (for isolated work in a separate directory).

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Requested name for the worktree |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Response family: WorktreeResponse

This is a unique response family — the hook must return the absolute path where the worktree was created.

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Path | 0 | absolute path string | — | Claude uses this path as the worktree location |

The stdout is a plain string (the absolute path), not JSON.

### Matcher field
None — WorktreeCreate does not support matchers.

### Settings registration

```json
{
  "hooks": {
    "WorktreeCreate": [
      {
        "type": "command",
        "command": "bash .claude/hooks/create-worktree.sh"
      }
    ]
  }
}
```

### Use cases
- Custom worktree creation logic (specific parent directories, naming conventions)
- Integration with project-specific worktree tooling (e.g., `git-wt`)
- Logging worktree creation for cleanup tracking

---

## WorktreeRemove

### When it fires
When Claude requests removal of a git worktree.

### Input fields

| Field | Type | Description |
|-------|------|-------------|
| `worktree_path` | string | Absolute path to the worktree to remove |
| + common fields | | `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id?`, `agent_type?` |

### Response family: SideEffectResponse

| Option | Exit code | Stdout | Stderr | Effect |
|--------|-----------|--------|--------|--------|
| Done | 0 | — | — | Removal proceeds |
| Context | 0 | `{"hookSpecificOutput":{"additionalContext":"..."}}` | — | Removal proceeds with context injected |

### Matcher field
None — WorktreeRemove does not support matchers.

### Settings registration

```json
{
  "hooks": {
    "WorktreeRemove": [
      {
        "type": "command",
        "command": "bash .claude/hooks/cleanup-worktree.sh"
      }
    ]
  }
}
```

### Use cases
- Custom cleanup logic before worktree removal
- Verify no uncommitted changes before removing
- Log worktree lifecycle for auditing
