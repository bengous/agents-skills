---
name: goalify
description: Use this skill when the user wants to convert a rough coding, product, architecture, audit, review, migration, cleanup, debugging, or orchestration intention into a compact Codex `/goal` command plus an optional first-turn session prompt. Trigger on "goalify", "Codex goal", "/goal", "make this a goal", "RALF loop", "turn this into a Codex prompt", or "meta-prompt for Codex". Do not use for normal implementation or review unless the user explicitly asks to produce a Codex goal/prompt.
license: MIT
compatibility: Designed for Codex CLI 0.128+ with GPT-5.5 and the Agent Skills format. Instruction-only; no scripts or network access required.
metadata:
  version: "0.1.0"
  author: "bengous"
---

# Goalify

Convert a messy human intention into the smallest effective Codex `/goal` setup.

You are not doing the downstream implementation. You are generating the command and optional first-turn prompt that the human can paste into a separate Codex session.

Do not create, set, update, pause, resume, clear, or complete an actual goal yourself. Output text only.

## Core principle

`/goal` is the durable destination of the session, not the whole runbook.

Keep durable outcome, acceptance criteria, validation, and stop conditions in `/goal`.

Put short-lived execution guidance in the session prompt.

Put recurring project conventions in `AGENTS.md` or another skill.

Omit or link large context.

## Inputs to extract

From the user's intention, extract only what changes the generated prompt:

- task type: implementation, audit, review, debugging, cleanup, migration, research, PRD/planning, orchestration, or continuation
- deliverable
- scope boundaries
- known files, branches, diffs, issue numbers, or artifacts
- acceptance criteria
- validation commands or evidence
- risk level and side-effect constraints
- manual approval gates
- desired stopping condition
- blockers that should pause the goal

If information is missing, make a reasonable assumption unless the missing detail materially changes scope, risk, or acceptance criteria. Ask at most three narrow clarification questions only when necessary. When possible, still provide a usable draft with explicit assumptions.

## Context placement rubric

Classify context before writing the output:

| Placement | Use for |
|---|---|
| `put_in_goal` | durable outcome, observable done criteria, validation, stop/pause condition |
| `put_in_session_prompt` | immediate first step, files to inspect first, test loop, implementation constraints, reporting format |
| `put_in_agents_md_or_skill` | recurring coding standards, team conventions, TDD policy, commit discipline, reusable orchestration rules |
| `omit_or_link` | long logs, pasted diffs, full PRDs, transcripts, giant file lists, repeated background, speculative context |

Never put `put_in_agents_md_or_skill` or `omit_or_link` content into `/goal`.

## `/goal` construction rules

Generate exactly one recommended `/goal` command unless the task is too broad for one coherent unit. If it is too broad, produce 2-4 candidate goals and mark the recommended first goal.

The recommended `/goal` line must be:

- outcome-first
- scoped to one coherent unit of work
- written as a final state, not a procedure
- explicit about "done"
- explicit about validation or evidence
- explicit about when to stop or pause
- concise enough to remain useful across turns
- free of full implementation strategy
- free of long file lists, full logs, full diffs, pasted PRDs, or transcripts

Default maximum length: 900 characters. Prefer 350-650 characters for normal tasks.

Use this shape by default:

```text
/goal <deliverable>. Success means <observable acceptance criteria>. Validate with <commands or evidence>. Stop when <reviewable stopping condition>; pause and report blockers if <blocking condition>.
```

If the task is plan-only, audit-only, or review-only, say so in the goal and prohibit edits unless the user asked for them.

If the task involves destructive actions, production systems, secrets, billing, permissions, data deletion, migrations, or external side effects, include an explicit human approval gate.

## Session prompt rules

Generate a `session_prompt` only when useful.

The session prompt may include:

- first action
- concise repo/context hints
- files or commands to inspect first
- TDD or validation loop
- manual approval gates
- scope boundaries
- expected final report format
- artifact paths

The session prompt must not duplicate durable repo conventions that belong in `AGENTS.md`.

Keep the session prompt compact. Prefer 100-250 words.

## GPT-5.5 defaults

Assume GPT-5.5 unless the user requests another model.

Prefer outcome-first instructions over step-heavy prompting.

Use `medium` reasoning as the default for ordinary coding/planning work. Suggest higher effort only for hard architecture, multi-repo migrations, security-sensitive analysis, or difficult debugging where quality justifies cost. Suggest lower effort only for straightforward prompt shaping.

Keep output concise and information-dense.

## Workflow defaults

Unless the user says otherwise:

- prefer TDD for implementation tasks
- inspect existing project instructions before proposing changes
- keep changes scoped and reviewable
- validate after each meaningful milestone
- avoid adjacent features
- do not commit unless explicitly requested
- do not perform destructive or external side-effect actions without explicit approval
- use subagents only when the work is independent and parallel review materially improves quality
- if the task is too broad, split it and recommend the first slice

## Output format

Return exactly these sections, in this order.

```markdown
## Recommended command

```text
/goal ...
```

## Session prompt

```text
...
```

## Context placement

| Content | Placement | Reason |
|---|---|---|
| ... | put_in_goal / put_in_session_prompt / put_in_agents_md_or_skill / omit_or_link | ... |

## Validation contract

- ...

## Stop rules

- ...

## Notes

- ...
```

If no session prompt is needed, write:

```text
No extra session prompt needed.
```

If clarification is required, add this section before `Recommended command`:

```markdown
## Clarification needed

1. ...
2. ...

Assumptions used for the draft:
- ...
```

## Quality check before finalizing

Before returning the output, verify:

- The recommended command starts with `/goal`.
- The goal is one coherent destination.
- The goal does not contain a runbook.
- The goal includes done criteria.
- The goal includes validation or evidence.
- The goal includes stop/pause behavior.
- Large context is linked, summarized, or omitted.
- Manual approval is required for destructive or external side-effect work.
- The session prompt contains only task-local execution guidance.
- The output is directly pasteable into Codex.

## Examples

### Implementation

User intention:

```text
I need Codex to add Codex-provider support to the orchestrator without breaking Claude support. It should be TDD and stop before commit.
```

Output:

```markdown
## Recommended command

```text
/goal Add provider-agnostic Codex support to the orchestrator while preserving existing Claude behavior. Success means provider selection is explicit, existing Claude flows still pass, Codex flows have targeted tests, and no unrelated architecture changes are included. Validate with the smallest relevant unit/integration test set plus existing orchestrator checks. Stop when the diff is reviewable and tests pass; pause and report blockers if provider semantics are ambiguous or parity cannot be proven.
```

## Session prompt

```text
Start by reading AGENTS.md and the orchestrator/provider code paths. Identify the current Claude-specific seams, then write or update tests for provider selection and Codex behavior before implementation. Keep the diff narrow. Do not commit. Final report: changed files, validation commands/results, remaining parity gaps.
```
```

### Audit only

```text
/goal Audit the current provider-agnostic orchestration branch for parity gaps between Claude and Codex without editing files. Success means a concise report identifies confirmed gaps, false alarms, risky assumptions, and recommended next slices. Validate by inspecting the diff against main and reading the relevant provider, hook, and session-forking code paths. Stop when each finding has file-level evidence; pause if required files or branch context are unavailable.
```

### Planning / PRD

```text
/goal Produce a reviewable PRD and slice plan for the proposed workflow, not implementation. Success means the plan defines scope, non-goals, user-visible behavior, architecture boundaries, validation strategy, manual approval gates, and first implementation slice. Validate by cross-checking against AGENTS.md and existing project constraints. Stop when the PRD is ready for human review; pause if product intent or risk tolerance is materially ambiguous.
```

### Debugging

```text
/goal Diagnose and fix the failing behavior with the smallest safe change. Success means the root cause is identified, a regression test fails before the fix and passes after it, and no unrelated files are changed. Validate with the targeted failing test plus the nearest relevant suite. Stop when the fix is minimal and tests pass; pause if reproduction is impossible or the failure points to a larger design issue.
```
