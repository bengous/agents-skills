---
name: ci-optimize
description: Make CI faster and cheaper in runner minutes, on any stack and provider. Use when CI jobs are slow, runner minutes or queue times grow, a job got slower, or before changing CI config for speed.
license: "MIT. Copyright (c) 2026 Augustin BENGOLEA. Full terms in LICENSE at https://github.com/bengous/agents-skills"
---

# CI optimization

Two costs, always counted apart: **wall time** to feedback (the critical path of the pipeline) and **billed minutes** (every job, every re-run). A lever can cut one and raise the other. The agent brings numbers and a ranked diagnosis; the user picks the levers.

## Workflow

1. **Baseline.** Rank the jobs by the minutes they consume over a window (duration times runs), then pull each top job's durations and queue time over recent runs. On self-hosted or shared runners, also read the host's load per run and split the spread into intrinsic cost and contention. Done when each top job has a median, a spread, a queue time and the run count behind them, plus the intrinsic and contention shares where load is readable. Sources, and how to split intrinsic cost from contention: [measure.md](references/measure.md)
2. **Decompose.** Mark the pipeline's critical path through its job graph, then break each slow job into steps, scripts and the tool's own phases (from the job log). Done when the largest cost of each job, and of the critical path, has a name and a number of seconds. Timing sources and how parallel work costs: [measure.md](references/measure.md)
3. **Explain.** Find the cause of each named cost, and measure its marginal cost: locally for a local cause (the tool with and without the suspect), otherwise from the logs and the run API. Done when every cause is measured or read in a log, none assumed. Each cause's signs, how to confirm it, its fixes and its proof: [levers.md](references/levers.md)
4. **Diagnose with the user.** Present a table of causes with the gain in wall time and in billed minutes, each counted separately, what each fix needs, and how each will be proven. Discuss before writing code. Done when the user has picked the levers; with no user to ask, implement the top-ranked lever only and say so.
5. **Research before building.** Done when each picked lever names its mechanism and the source behind it (a project file, a doc URL, an audited tool pinned to a commit), with each rejected option and one reason. Search order, the third-party audit brief, prerelease features: [tools.md](references/tools.md)
6. **Prototype, then implement.** Try the change in a throwaway form first (a local run, a temporary worktree, a prerelease binary in a scratch directory). Done when the prototype moved the number in the predicted direction and that lever is its own commit naming the measured cause; a lever whose prototype shows no gain goes back to the user with its numbers, not into a commit.
7. **Prove it.** Run the proving stages that fit the lever, from local tests to the first run after the merge. Done when each lever has a before and after number from CI, with its conditions. The stages, in order, and how to wait on CI: [validate.md](references/validate.md)
8. **Follow up.** A question that needs time (cache growth over weeks, an upstream release) gets a follow-up before the session ends: a reminder or scheduled task where the harness has one, otherwise an issue or a TODO entry. Done when every open question has a follow-up with its trigger, the last numbers and the commands to re-measure.

## Delegation

- Independent research questions run in parallel read-only research subagents (`swarm-research` when installed).
- A third-party tool gets the audit in [tools.md](references/tools.md).
- Changes that live in another repository (shared workflows, runner config) go to an agent session in that repository when the harness can start one; otherwise list them for the user.
- Measurements you will compare stay inline, on one machine under one load.

## Principles

- **Measure before you cut.** A suspicion becomes a lever once a number backs it.
- **The cheapest job is the one that never runs.** Prefer a lever that removes work (a skip, a dropped duplicate, a narrower scope) over one that speeds it up.
- **Proof over trust.** Judge a change by evidence that the work it claims happened (a hit count, a step's conclusion, a log line), never by a green badge.
- **Fail open.** Optimization logic that cannot decide runs the full work; an API error never turns a job red.
- **One owner per check.** Each check runs once per change, in the job that owns it; local commands keep their full chain.
- **A number's conditions.** Every figure carries its source (run id, log line, command), the load and the machine; a single run is reported as one run. "With its conditions" means all of these, wherever this skill says it.
