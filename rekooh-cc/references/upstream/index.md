# Upstream — Official Docs and Contract Reference

Synced official documentation and distilled contract references.

## Files in this directory

| File | Description |
|------|-------------|
| [contracts-by-event.md](contracts-by-event.md) | Compact cheat sheet of all 18 events — input fields, response families, exit codes, matcher fields |
| hooks-reference.md | Synced from official Claude Code hooks reference (managed by `scripts/sync-official-docs.ts`) |
| hooks-guide.md | Synced from official Claude Code hooks guide (managed by `scripts/sync-official-docs.ts`) |
| settings-reference.md | Synced from official Claude Code settings reference (managed by `scripts/sync-official-docs.ts`) |

## Syncing official docs

To refresh the upstream docs:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/sync-official-docs.ts
```

The script fetches the latest official documentation and prepends retrieval metadata (source URL, fetch date) to each file.

## When to use upstream docs

- **Verifying contracts** — When you need to confirm the exact behavior Claude Code expects from a hook
- **Checking for changes** — After a Claude Code update, sync and diff to find contract changes
- **Teaching** — When explaining the official hook system to a user unfamiliar with it

For most hook authoring tasks, the [events reference](../events/index.md) and [contracts-by-event cheat sheet](contracts-by-event.md) are more practical starting points than the raw official docs.
