# Inventory Commands

There is no bundled inventory script in v1. Use these hard-coded read-only
commands as the current script substitute. Adapt paths for non-Bun or unusual
repos.

## Baseline

```bash
pwd
git status --short --branch
git rev-parse --show-toplevel
git ls-files | wc -l
```

## Agent Context And Projections

```bash
find . -path './.git' -prune -o \
  \( -name AGENTS.md -o -name CLAUDE.md -o -name AI.md \) -print | sort

find .claude -maxdepth 4 -type f 2>/dev/null | sort
find .agents -maxdepth 4 -type f 2>/dev/null | sort
find .codex -maxdepth 4 -type f 2>/dev/null | sort
```

## Skills

```bash
find .agents/skills .claude/skills .codex/skills -maxdepth 4 -type f 2>/dev/null | sort
find .agents/skills .claude/skills .codex/skills -name SKILL.md -maxdepth 4 -type f 2>/dev/null | sort
```

## Hooks, MCP, Runtime Config

```bash
find . -path './.git' -prune -o \
  \( -path './.agents/hooks/*' -o -path './.codex/hooks/*' -o -path './.claude/hooks/*' -o -name '.mcp.json' -o -name 'settings.json' -o -name 'config.toml' \) -print | sort

rg -n "mcp|hook|permission|sandbox|approval|timeout|danger|destructive|generated|source of truth|AGENTS.md|CLAUDE.md" \
  .agents .codex .claude docs scripts package.json 2>/dev/null
```

## Validation And CI

```bash
rg -n '"scripts"|validate|check|typecheck|lint|test|e2e|security|audit|eval|agents:sync|agents:check' \
  package.json bunfig.toml mise.toml .github scripts 2>/dev/null

find .github scripts -maxdepth 4 -type f 2>/dev/null | sort
```

## Generated Output And Sync

```bash
rg -n "sync|generated|manifest|checksum|template|template-sources|copy|projection|preserve" \
  scripts src template-sources templates docs package.json 2>/dev/null

find template-sources templates config -maxdepth 4 -type f 2>/dev/null | sort
```

## Continuity Artifacts

```bash
find . -path './.git' -prune -o \
  \( -iname '*prd*' -o -iname '*plan*' -o -iname '*tracker*' -o -iname '*status*' -o -iname '*handoff*' -o -iname '*adr*' \) -print | sort

find docs .itw .agents/reports -maxdepth 4 -type f 2>/dev/null | sort
```

## Optional Cheap Validation

Run validation only when it is known, cheap, and relevant to the audit. Examples:

```bash
bun run agents:check
bun run parent-tooling:check
bun run check
```

Do not turn audit into implementation. Validation failures are evidence; fixing
them is a separate request.
