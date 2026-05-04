# Phase Contract

Use this file only after `$orchestration-planner` is invoked and a root is known.

## Universal Gate

Start with:

```bash
orch status <root>
```

If the reported stage is not the stage you are about to work on, stop. Report the
actual stage and ask the human how to proceed.

At the end of a phase:

1. say explicitly that the phase is complete;
2. summarize only the relevant artifact path;
3. ask the human to run `orch advance <root>`.

## intake

Goal: capture the initial intention and obvious assumptions in `intake.md`.

If intention is missing, ask the user for it. Do not start clarification until
the human advances.

Done when `intake.md` contains a usable intention.

## clarification

Use `$grill-me`.

Goal: remove ambiguity before PRD generation.

Maintain `clarification.md` as an append-only Q/R/reco/decision log. Write the
current answer and decision before asking the next question.

Done when there are no material product/workflow ambiguities left for a first
PRD draft.

## prd

Use `$to-prd`.

Goal: write `prd.md` from `intake.md` and `clarification.md`.

Local-only constraint: if `$to-prd` would publish or create a GitHub issue, do
not follow that part. Produce only the local `prd.md`.

Keep open implementation details as open questions or follow-ups instead of
blocking the PRD when the product contract is clear.

Done when `prd.md` has core sections and is ready for human review.

## prd_review

Human gate. Do not create issues. Wait for human approval and `orch advance`.

## issues

Use `$to-issues`.

Goal: write vertical implementation slices in `issues.md`.

Local-only constraint: if `$to-issues` would create GitHub issues, do not follow
that part. Produce only the local `issues.md`.

Minimum issue fields:

- title
- `Type: AFK | HITL`
- `Depends on`
- `Goal`
- `Acceptance`
- `TDD`
- `Validation`
- `Agent`

Generated worker prompts must invoke `$tdd`; do not inline a custom TDD contract.

Done when issue granularity and dependencies are reviewable.

## issues_review

Human gate. Do not create workflow artifacts. Wait for human approval and
`orch advance`.

## workflow

Goal: finalize `workflow.md`, `tracker.md`, `prompts/`, and optional reviewer or
agent files.

Use `$deepen-codebase-architecture` only if the PRD/issues reveal unclear module
boundaries or high architecture risk.

The workflow must be self-contained enough for a different execution agent to
run later.

Done when the workflow package is ready for human review.

## workflow_review

Human gate. Do not launch execution. Wait for human approval and `orch advance`.

## workflow_ready

The final `orch advance` prints a concise handoff prompt. Give that prompt to an
execution agent. Do not launch workers from this skill.
