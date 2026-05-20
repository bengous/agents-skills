---
name: docs-drift
description: "Audit Codex-facing repo guidance for drift against recent code changes. Use when the user invokes $docs-drift, asks whether AGENTS.md, AGENTS.override.md, .agents/skills, .codex/config.toml, project skill guidance, or other Codex instructions are stale, after a batch of commits, before shipping reusable agent workflows, or when documentation/guidance may no longer match the code. Optimized for GPT-5.5/Codex: inspect real files first, use official OpenAI Codex docs or live web search only for current Codex behavior, and produce evidence-backed findings before edits."
---

# Docs Drift

Audit whether Codex-facing instructions still match the repo.

This skill is report-first. Do not edit guidance unless the user explicitly asks
to apply fixes after seeing proposed changes.

## Scope

Check only guidance that changes Codex behavior:

- `AGENTS.md` and `AGENTS.override.md` files along the repo path
- repo skills under `.agents/skills/` or `*/SKILL.md` when the repo packages skills
- project-local `.codex/config.toml`, `hooks.json`, MCP/app/plugin config, and similar Codex runtime surfaces
- repo docs that are explicitly imported or referenced by those guidance files

Do not audit ordinary README/API docs unless they are used as Codex instructions.

## Workflow

1. Run the bundled scanner from the target repo:
   ```bash
   python <skill-dir>/scripts/codex_docs_drift.py
   ```
   Add `--since <rev>` when the user gives a baseline. Add `--json` only for
   machine processing.
2. Read the generated report before opening files. It identifies the instruction
   baseline, drift window, changed zones, and candidate guidance files.
3. Inspect real files. For each candidate, compare current guidance to changed
   source, tests, scripts, config, and generated-source boundaries.
4. Use official Codex docs only when current Codex behavior matters. Prefer
   OpenAI developer docs MCP; otherwise use web search restricted to official
   OpenAI Codex docs. Use `references/codex-guidance.md` for the exact doc
   topics worth checking.
5. Produce findings first. Use this shape:
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
6. If the user approves edits, patch only the relevant source guidance files.
   Keep changes concise: stable constraints, ownership boundaries, validation
   commands, and source-of-truth rules. Avoid volatile line-by-line procedures.

## GPT-5.5 Operating Rules

- Spend the large context on source evidence, not copied docs. Load docs only
  for behavior that may have changed.
- Prefer one deep source-backed pass over many shallow findings.
- Mark uncertainty explicitly when source evidence is partial.
- Treat stale guidance as a contract bug only when it can mislead future agents.
- Do not create compatibility notes for old Codex behavior unless the repo still
  supports that behavior.

## Drift Heuristics

Flag likely drift when recent commits:

- add, rename, or remove modules mentioned by guidance;
- move source-of-truth ownership between runtime config, generated files, repo
  source, live install, or external systems;
- change validation commands, package managers, hooks, sandbox assumptions, or
  deployment/install paths;
- introduce a mature local pattern that guidance still tells agents to recreate;
- touch skill metadata, `SKILL.md`, scripts, or agent config without updating
  corresponding trigger and validation guidance.

Ignore:

- pure prose polish with no agent behavior impact;
- generated files when their source still matches the guidance;
- implementation details that code already makes obvious.

## Resources

- `scripts/codex_docs_drift.py`: deterministic report generator for drift window
  and Codex guidance candidates.
- `references/codex-guidance.md`: official Codex documentation checkpoints and
  source URLs consulted when this skill was written.
