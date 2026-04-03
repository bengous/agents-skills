---
name: rekooh-cc
description: Use when the user explicitly asks to create, modify, audit, or understand Claude Code hooks in any repository. Covers hook strategy, scaffolding, settings registration, testing, and the typed runtime.
disable-model-invocation: true
---

# rekooh — Claude Code Hooks Authoring

Build, audit, and manage Claude Code hooks for any project.

## Quick Start

1. **Audit** — Run `bun scripts/inspect-project.ts --dir <project>` to assess current state
2. **Bootstrap** — Run `bun scripts/bootstrap-hooks-infra.ts --dir <project>` to create `.claude/hooks/`
3. **Scaffold** — Run `bun scripts/scaffold-standalone-hook.ts --event <Event> --lang bash --name <name> --dir <project>`
4. **Register** — Run `bun scripts/patch-settings.ts --event <Event> --command .claude/hooks/<name>.sh --dir <project>`

All scripts run from the skill root: `~/.claude/skills/rekooh-cc/`.

## Scripts

| Script | Purpose |
|--------|---------|
| `inspect-project.ts` | Analyze project, detect runtimes, recommend strategy |
| `bootstrap-hooks-infra.ts` | Create `.claude/hooks/` dir (add `--opinionated` for typed runtime) |
| `scaffold-standalone-hook.ts` | Generate hook from template (`--lang bash\|python\|bun`) |
| `scaffold-opinionated-hook.ts` | Generate typed hook using `defineHook` |
| `patch-settings.ts` | Register hook in `.claude/settings.json` (idempotent) |
| `validate-hooks.ts` | Verify all registered hooks exist and are executable |
| `sync-official-docs.ts` | Fetch latest upstream docs from code.claude.com |

## Strategy

| Approach | When to use |
|----------|-------------|
| Standalone bash | Simple guards, no runtime deps, broadest compatibility |
| Standalone python | Complex logic, python already in project |
| Standalone bun | Type-safe without framework overhead |
| Typed runtime (`defineHook`) | Multiple hooks, wants testing + response type safety |

Read [references/strategy/index.md](references/strategy/index.md) for the full decision tree.

## References

### Workflows
| Need | Reference |
|------|-----------|
| Audit existing setup | [references/audit/index.md](references/audit/index.md) |
| Choose a strategy | [references/strategy/index.md](references/strategy/index.md) |
| Bootstrap infrastructure | [references/bootstrap/index.md](references/bootstrap/index.md) |
| Write or modify hooks | [references/authoring/index.md](references/authoring/index.md) |
| Configure settings.json | [references/config/index.md](references/config/index.md) |
| Test hooks | [references/testing/index.md](references/testing/index.md) |

### Event Reference
| Need | Reference |
|------|-----------|
| All 18 events overview | [references/events/index.md](references/events/index.md) |
| PreToolUse (guard/modify tools) | [references/events/pretooluse.md](references/events/pretooluse.md) |
| PostToolUse (react to tool results) | [references/events/posttooluse.md](references/events/posttooluse.md) |
| Stop / SubagentStop / TeammateIdle | [references/events/stop.md](references/events/stop.md), [references/events/subagent.md](references/events/subagent.md) |
| Session lifecycle | [references/events/lifecycle.md](references/events/lifecycle.md) |
| All events contract cheat sheet | [references/upstream/contracts-by-event.md](references/upstream/contracts-by-event.md) |

### Advanced
| Need | Reference |
|------|-----------|
| Typed runtime overview | [references/opinionated-lib/index.md](references/opinionated-lib/index.md) |
| Step-by-step defineHook guide | [references/opinionated-lib/create-hook-guide.md](references/opinionated-lib/create-hook-guide.md) |
| Runtime module map | [references/opinionated-lib/runtime-map.md](references/opinionated-lib/runtime-map.md) |
| Official upstream docs | [references/upstream/index.md](references/upstream/index.md) |

### Examples
| Example | Event | Approach |
|---------|-------|----------|
| [Block destructive bash](examples/block-destructive-bash/index.md) | PreToolUse | Bash |
| [Protect paths](examples/protect-paths/index.md) | PreToolUse | Bash |
| [Run lint after edit](examples/run-lint-after-edit/index.md) | PostToolUse | Bash |
| [Stop notification](examples/stop-notification/index.md) | Stop | Bash |
| [Typed hook](examples/typed-hook-with-opinionated-lib/index.md) | PreToolUse | defineHook |
| [Full bootstrap walkthrough](examples/complete-project-bootstrap/index.md) | — | All scripts |
