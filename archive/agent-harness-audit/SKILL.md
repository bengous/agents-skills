---
name: agent-harness-audit
description: "Audit a repository's agent harness and context engineering. Use when the user asks to audit, review, score, compare, or improve the repo's AI/agent setup: instructions, AGENTS/CLAUDE files, skills, hooks, MCP, validation, permissions, generated projections, continuity, traces, and source-of-truth boundaries. Produces an evidence-backed Markdown audit report; does not implement changes unless separately requested."
---

# Agent Harness Audit

Audit the repository-local system that helps agents work reliably: context,
instructions, generated projections, skills, hooks, tools, MCP, validation,
permissions, continuity, traces, and source-of-truth ownership.

## Modes

- `audit` is the default. Inspect the repo, produce advice, and write a local
  Markdown audit artifact unless the user asks for conversation-only output.
- `corpus refresh` is explicit only. Use it when the user asks to update the
  source corpus, latest recommendations, or audit grid. It may edit the skill's
  references, but only after the user clearly requests maintenance.

## Boundaries

- Do not implement harness changes during `audit`.
- Do not edit generated projections directly when a source exists.
- Do not inspect global/runtime agent config unless the user explicitly asks for
  runtime audit.
- Do not send external messages, install tools, push, publish, or apply live
  config.
- Report missing evidence as missing evidence; do not fill gaps with generic
  best-practice claims.

## References

Read only the references needed for the request:

- `references/terminology.md`: canonical terms and ownership language.
- `references/audit-grid.md`: surfaces, severity, confidence, and adaptive audit
  routing.
- `references/inventory-commands.md`: hard-coded read-only shell inventory.
  There is no bundled script in v1.
- `references/source-corpus.md`: source freshness, corpus refresh, and evidence
  labels.
- `references/kitsmith-context-strategy.md`: use when the repo follows a
  Kitsmith-style context/projection model.
- `references/report-template.md`: required report shape.

## Audit Workflow

1. Identify the target repo, requested mode, intensity, and artifact path.
   Default intensity is `standard`; accepted values are `fast`, `standard`, and
   `deep`.
2. Announce the artifact side effect before writing. Default path:
   `.agents/reports/harness-audit-YYYY-MM-DD.md`.
3. Read `audit-grid.md`, `inventory-commands.md`, `source-corpus.md`, and
   `report-template.md`. Add `kitsmith-context-strategy.md` when the repo has
   `CLAUDE.md`, `.claude/rules/**`, generated `AGENTS.md`, or Kitsmith tooling.
4. Run a read-only inventory. Prefer the command set in
   `inventory-commands.md`; adapt it to the repo instead of adding scripts.
5. Classify the harness as `small`, `medium`, or `large`.
6. For `small` or `fast`, audit inline. For `standard` or `deep`, use read-only
   audit forks/subagents only when the current runtime and instructions allow
   delegation. If delegation is unavailable, audit inline and document the
   coverage limit.
7. Aggregate evidence, deduplicate findings, resolve contradictions, and write a
   single report. Recommendations must cite local evidence and label the source
   basis: local proof, bundled corpus, freshly verified source, or inference.
8. Final response: give the report path, top findings, limits, and validation
   performed. Do not produce implementation slices unless the user separately
   asks for implementation planning.

## Corpus Refresh Workflow

Use only on explicit request. Read `source-corpus.md` first. Verify date-sensitive
claims against primary sources, prefer official documentation, and summarize the
sources consulted. Leave a visible Git diff for reference updates.
