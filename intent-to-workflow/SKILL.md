---
name: intent-to-workflow
disable-model-invocation: true
description: >
  Human-invoked meta-skill for turning a broad intention into staged local
  artifacts: intake, clarification, terminology, PRD, issues, workflow,
  tracker, prompts, and final execution handoff. Use only when the user explicitly invokes
  $intent-to-workflow or asks to scaffold this PRD-to-issues-to-workflow
  process with human gates.
---

# Intent To Workflow

Plan a future execution workflow. Do not execute that workflow.

The deterministic gate is `itw`:

```bash
itw init <root> <intention...>
itw status <root>
itw get <root>
itw advance <root>
```

Always run the gate. Use `itw`; if missing, use the installed skill path:
`<skill-dir>/scripts/itw` on Unix or `& '<skill-dir>\scripts\itw.ps1'` on
Windows. Never hand-roll workflow state. Stop on launcher failure.

## Operating Rules

- Human-invoked only. Do not activate implicitly.
- `$intent-to-workflow` requires an explicit initial intention. Empty invocation
  creates no root unless the current session already has one; same-session
  reinvocation resumes with `itw get`.
- `intake` is raw extensionless text captured from the initial intention. Do not
  rewrite it; put interpretation and corrections in `clarification.md`.
- `terminology.md` is the local language model for actors, roles, canonical
  terms, relationships, and ambiguities. Keep it current when clarification
  changes understanding; do not update it mechanically after every answer.
- `itw status <root>` is compact human status.
- `itw get <root>` is the agent-facing phase prompt and recovery surface.
- At the start of every phase, run `itw get <root>` or ask the human for the root.
- English is the default state language. If `intake` is clearly not English,
  run `itw set-language <root> <language-code>` before continuing, then follow
  the refreshed `itw get <root>` prompt.
- `en` is the native artifact language. `fr` is structurally localized for
  `clarification.md`, `terminology.md`, and `prd.md`: human-facing prose and
  Markdown headings/body in those artifacts use French.
- Other supported language codes are instruction-only until localized templates
  exist: write human-facing prose in that language inside artifact sections, but
  keep English Markdown headings, file names, commands, code identifiers,
  required machine field labels, and canonical product/technical terms.
- `issues.md`, `workflow.md`, `tracker.md`, and `prompts/*.md` keep their
  established English structure and machine-facing labels.
- In `terminology.md`, canonical product/technical terms may be English, but
  definitions, relationships, ambiguities, and aliases to avoid follow the
  workflow language.
- Work only inside the current phase.
- End each phase by saying the phase is complete and asking the human to run
  `itw advance <root>`.
- Never run `itw advance` yourself unless the user explicitly overrides the
  human-gate contract.
- Keep planning artifacts under `itw/<id>/` unless the user chose another root.
- `tracker.md` tracks the future execution workflow, not this planning process.
- Do not write artifacts outside the workflow root from this skill.

## Phase Map

1. `clarification`
2. `prd`
3. `prd_review`
4. `issues`
5. `issues_review`
6. `workflow`
7. `workflow_review`
8. `workflow_ready`

## Owned Prompt Workflow

Do not invoke companion skills for the phase workflow. `itw get` and
`itw advance` inject the current phase prompt plus only the current phase
reference from packaged resources.

Packaged references live under `src/intent_to_workflow/references/`:

- `grill.md`
- `prd.md`
- `prd_review.md`
- `issues.md`
- `issues_review.md`
- `tdd.md`
- `artifacts.md`

For the optional Codex `UserPromptSubmit` hook and installation notes, read
[references/hook.md](references/hook.md).
