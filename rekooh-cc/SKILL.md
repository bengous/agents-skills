---
name: rekooh-cc
description: >
  Claude Code hooks authoring and audit skill. Use when creating, auditing, modifying,
  or explaining Claude Code hooks, .claude/settings.json hook configuration, hook
  events, handler types, matchers, testing, debugging, or official hooks examples.
  Focuses on current official docs plus small local scripts; not for generic Git hooks.
disable-model-invocation: true
model: opus
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
  - WebFetch
  - WebSearch
---

# Rekooh

Use this skill to build or audit Claude Code hooks with the smallest reliable
implementation: current official docs, local repo inspection, copyable examples,
and a few deterministic scripts.

## Doctrine

Claude Code owns the hook runtime contract. This skill owns the authoring
workflow around that contract.

Prefer boring command hooks for deterministic local automation. Reach for HTTP,
MCP, prompt, or agent handlers only when the official docs and the project shape
justify them. Do not invent a hook when a project script, README instruction, or
agent prompt is enough.

## Not For

- Generic Git hooks such as `pre-commit`, `post-merge`, or `post-rewrite`
- Product or business logic hidden in agent hooks
- Broad security promises beyond deterministic local guardrails
- Building a custom hook framework before a plain script proves insufficient

## Workflow

1. Inspect local truth first: `.claude/settings.json`, `.claude/settings.local.json`,
   `.claude/hooks/`, package scripts, and existing agent instructions.
2. Classify the request: audit, explain, create, modify, register, test, or debug.
3. For contract details, consult the official docs before relying on memory.
4. Choose the smallest handler type and storage scope that solves the problem.
5. If changing files, implement narrowly and run direct hook smoke tests plus
   settings validation.
6. Report the exact files changed and any official docs consulted.

## Official Docs

Start here when contract details matter:

- Hooks reference: `https://code.claude.com/docs/en/hooks`
- Hooks guide: `https://code.claude.com/docs/en/hooks-guide`
- Settings reference: `https://code.claude.com/docs/en/settings`
- Docs map: `https://code.claude.com/docs/llms.txt`

## Local Scripts

Run scripts by absolute path from this skill root. They are intentionally small
and safe to inspect before use.

| Intent | Command |
|---|---|
| Inspect a project | `bun <skill-root>/scripts/inspect-project.ts --dir <project>` |
| Register a command hook | `bun <skill-root>/scripts/patch-settings.ts --settings <settings.json> --event PreToolUse --matcher Bash --command ".claude/hooks/guard.sh"` |
| Validate hook config | `bun <skill-root>/scripts/validate-hooks.ts --settings <settings.json>` |
| Check official doc links | `bun <skill-root>/scripts/check-official-links.ts` |

`patch-settings.ts` is deliberately command-hook-only. For HTTP, MCP, prompt, or
agent handlers, use the official docs and edit settings explicitly.

## Navigation

Load only the reference needed for the current task.

| File | Answers |
|---|---|
| `references/official-docs.md` | Which official docs and external examples should be checked? |
| `references/decision-matrix.md` | Should this be a hook, and which handler/scope fits? |
| `references/configuration.md` | What does `.claude/settings.json` look like? |
| `references/events-and-outputs.md` | Which events and output decisions matter? |
| `references/patterns.md` | What code shape should a hook use? |
| `references/testing-debugging.md` | How do I test, debug, and prove a hook works? |
| `references/examples.md` | Copyable examples for common hook tasks |

## Guardrails

- Treat generated or copied hook code as project source once installed.
- Keep hook commands explicit; avoid hidden shell indirection unless necessary.
- Fail closed only for deterministic, high-confidence guards.
- Fail open for formatters, notifications, context helpers, and best-effort checks.
- Never silently swallow parse/config errors in auditing or validation tools.
- Keep local-only settings in `.claude/settings.local.json`; commit shared policy
  only when the team should inherit it.

## Inspiration

GBrain's agent-facing docs are useful as a shape reference: a small docs map,
explicit resolver/routing, and owned scaffolded skills. Do not copy its product
claims or brain-specific behavior; borrow the idea that agents need concise
entrypoints and links to deeper references.
