# Audit — Inspect Existing Hook Setup

Assess the current state of hooks in the target repository before making changes.

## Quick audit

Run the inspection script to get a JSON summary of the project's hook landscape:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/inspect-project.ts
```

The output includes:

| Field | Description |
|-------|-------------|
| `projectRoot` | Detected project root directory |
| `settingsFiles` | All `.claude/settings.json` and `.claude/settings.local.json` files found |
| `existingHooks` | Hook files already registered or detected by convention |
| `existingSkills` | Skills directories found under `.claude/skills/` |
| `availableRuntimes` | Which runtimes are available (bun, node, python, bash) |
| `packageManagers` | Detected package managers (bun, npm, yarn, pnpm) |
| `qualityCommands` | Linting/formatting commands found in package.json or config files |
| `recommendedStrategy` | Suggested hook strategy based on project context |
| `rationale` | Explanation of why the strategy was recommended |

## Manual audit checklist

1. **Find settings files** — Check for `.claude/settings.json` at project root and any nested settings files. Also check `.claude/settings.local.json` for user-local overrides.

2. **List registered hooks** — In each settings file, look at the `hooks` object. Each key is an event name (e.g., `PreToolUse`, `PostToolUse`). Each value is an array of hook registrations with `type`, `command`, and optional `matcher`/`timeout` fields.

3. **Check for conflicts** — Multiple hooks on the same event run in parallel. If two hooks on the same event + matcher could produce contradictory decisions (one allows, one blocks), the most restrictive decision wins. Identify if this is intentional.

4. **Identify gaps** — Compare registered hooks against the project's needs:
   - Are destructive commands guarded? (`PreToolUse` on `Bash`)
   - Are sensitive paths protected? (`PreToolUse` on `Write`/`Edit`)
   - Is there post-edit validation? (`PostToolUse` on `Write`/`Edit`)
   - Are stop notifications configured? (`Stop` event)

5. **Check hook file health** — For each registered hook command:
   - Does the file exist at the specified path?
   - Is it executable (for shell scripts)?
   - Does it handle stdin JSON correctly?
   - Does it exit with appropriate codes (0=allow, 1=error, 2=block)?

## Next steps

- If no hooks exist: proceed to [strategy](../strategy/index.md) to choose an approach
- If hooks exist but need changes: see [modifying existing hooks](../authoring/modifying-existing-hooks.md)
- If hooks exist and need validation: see [testing](../testing/index.md)
