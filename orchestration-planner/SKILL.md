---
name: orchestration-planner
disable-model-invocation: true
description: >
  Human-invoked meta-skill for turning a broad idea into staged local artifacts:
  intake, clarification, PRD, issues, workflow, tracker, prompts, and final
  execution handoff. Use only when the user explicitly invokes
  $orchestration-planner or asks to scaffold this PRD-to-issues-to-workflow
  orchestration process with human gates.
---

# Orchestration Planner

Plan a future execution workflow. Do not execute that workflow.

The deterministic gate is `orch`:

```bash
orch init <root> [intention...]
orch status <root>
orch advance <root>
```

If `orch` is not installed globally, run it from this skill package with
`uv run orch ...`.

## Operating Rules

- Human-invoked only. Do not activate implicitly.
- At the start of every phase, run `orch status <root>` or ask the human for the
  root if unknown.
- If the stage is not the stage requested by the user, stop and report the
  current stage. Do not infer the next phase.
- Work only inside the current phase.
- End each phase by saying the phase is complete and asking the human to run
  `orch advance <root>`.
- Never run `orch advance` yourself unless the user explicitly overrides the
  human-gate contract.
- Keep planning artifacts under `orc/<id>/` unless the user chose another root.
- `tracker.md` tracks the future execution workflow, not this planning process.

## Phase Map

1. `intake`
2. `clarification`
3. `prd`
4. `prd_review`
5. `issues`
6. `issues_review`
7. `workflow`
8. `workflow_review`
9. `workflow_ready`

Read [references/phase-contract.md](references/phase-contract.md) when entering a
phase.

## Companion Skills

- `clarification`: use `$grill-me`
- `prd`: use `$to-prd`, but write only local `prd.md`
- `issues`: use `$to-issues`, but write only local `issues.md`
- generated worker prompts: invoke `$tdd`
- workflow architecture: use `$deepen-codebase-architecture` only when boundaries
  or module design are unclear

If a required companion skill is missing, block clearly. Do not substitute copied
instructions.

Companion skills are guidance, not authority to publish. If a companion skill's
native workflow would create GitHub issues, push, or write outside the local
`orc/<id>/` root, constrain it to the local artifact for the current phase.

## Artifact Contract

Read [references/artifact-contract.md](references/artifact-contract.md) before
editing generated artifacts or explaining their responsibility boundaries.

For the optional Codex `UserPromptSubmit` hook and installation notes, read
[references/hook.md](references/hook.md).
