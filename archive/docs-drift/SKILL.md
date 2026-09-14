---
name: docs-drift
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $docs-drift or asks to audit Codex guidance drift. Audits AGENTS.md, AGENTS.override.md, .agents/skills, .codex/config.toml, project skill guidance, and other Codex instructions against recent code changes. Inspect real files first; use official OpenAI Codex docs or live web only for current Codex behavior; report findings before edits."
---

# Docs Drift

Audit whether Codex-facing instructions still match the repo. Report first;
edit only after explicit approval.

## Scope

- `AGENTS.md` and `AGENTS.override.md` files along the repo path
- repo skills under `.agents/skills/` or `*/SKILL.md`
- project-local `.codex/config.toml`, `hooks.json`, MCP/app/plugin config
- repo docs that are explicitly imported or referenced by those guidance files

Skip README/API docs unless Codex instructions depend on them.

## Workflow

1. Run:
   ```bash
   python <skill-dir>/scripts/codex_docs_drift.py
   ```
   Use `--since <rev>` for an explicit baseline; `--json` for machines.
2. Read the report first: baseline, drift window, changed zones, candidate
   guidance files.
3. Inspect real files. Compare guidance to changed source, tests, scripts,
   config, generated-source boundaries, and install/runtime ownership.
4. Check official Codex docs only when current Codex behavior matters. Prefer
   OpenAI docs MCP; otherwise live web search restricted to official OpenAI
   Codex docs. See `references/codex-guidance.md`.
5. Report findings before patches:
   ```md
   ## Findings
   - HIGH|MEDIUM|LOW: <guidance file> drifts from <source evidence>
     Evidence: <file:line or commit/source inspection>
     Fix: <precise change>

   ## No-Drift Areas
   - <guidance file>: current because <evidence>

   ## Proposed Patch
   <only if edits were requested or the user asks for the patch>
   ```
6. On approval, patch only relevant source guidance. Prefer stable constraints,
   ownership, validation, and source-of-truth rules; avoid volatile procedures.

## Rules

- Spend context on source evidence, not copied docs.
- Prefer one deep source-backed pass to shallow findings.
- Mark uncertainty explicitly when source evidence is partial.
- Treat stale guidance as a contract bug only when it can mislead future agents.
- Do not add old-Codex compatibility notes unless the repo still supports them.

## Drift Heuristics

Flag drift when commits:

- add, rename, or remove modules mentioned by guidance;
- move source-of-truth ownership across runtime config, generated files, repo
  source, live install, or external systems;
- change validation, package managers, hooks, sandbox assumptions, or install paths;
- introduce a mature local pattern that guidance still tells agents to recreate;
- touch skill metadata, `SKILL.md`, scripts, or agent config without updating
  corresponding trigger and validation guidance.

Ignore pure prose polish, generated files whose source matches guidance, and
implementation details the code already makes obvious.

## Resources

- `scripts/codex_docs_drift.py`: drift window and guidance candidate report.
- `references/codex-guidance.md`: official Codex documentation checkpoints and
  refresh triggers.
