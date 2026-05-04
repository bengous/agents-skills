# Artifact Contract

## Responsibility Boundaries

- `intake.md`: initial intention, entry point, assumptions.
- `clarification.md`: append-only Q/R/reco/decision log.
- `prd.md`: approved product intent and constraints.
- `issues.md`: vertical implementation slices.
- `workflow.md`: execution contract.
- `tracker.md`: live execution authority for the future workflow.
- `prompts/*.md`: durable worker/reviewer prompts.
- `.orchestration-state.json`: minimal machine state only.

Do not use `.orchestration-state.json` as a journal. Do not use `tracker.md` to
track the planning workflow itself.

## Generated Workflow Package

`orch advance` creates the workflow package when moving from `issues_review` to
`workflow`:

```text
workflow.md
tracker.md
prompts/
```

`reviews/`, `evidence/`, and `agents/` are optional. Create them only when the
workflow needs reviewer output, evidence capture, specialized agents, runtime
launch support, differentiated tools, or explicit user request.

## tracker.md

Default statuses:

- `pending`
- `in_progress`
- `reviewing`
- `completed`
- `blocked`

Keep `tracker.md` compact. Reference prompt files instead of embedding full
prompts.

## Prompt Files

Worker prompts are autonomous context packets:

- why this work exists;
- files to read;
- active issue;
- scope limits;
- `$tdd` invocation;
- validation and local commit policy.

Reviewer prompts are read-only and should write or report findings separately.

## Commit Policy

Workers commit their scoped slice locally after validation and accepted reviews.
The execution orchestrator records commits in `tracker.md` and may commit only
as fallback if a worker forgot. Push only after the whole workflow completes.
