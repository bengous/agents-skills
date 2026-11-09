---
name: ci-optimize
description: Make CI faster and cheaper in runner minutes, on any stack and provider, hosted or self-hosted. Use when CI jobs are slow or got slower, when runner minutes, re-runs, timeouts or queue times grow, when runners are saturated, or before changing CI config for speed.
license: "MIT. Copyright (c) 2026 Augustin BENGOLEA. Full terms in LICENSE at https://github.com/bengous/agents-skills"
---

# CI optimization

Two costs, always counted apart: **wall time** to feedback (the critical path of the pipeline) and **runner minutes** (every job, every re-run, as the runner's owner pays for them). A lever can cut one and raise the other. The agent brings numbers and a ranked diagnosis; the user picks the levers.

## Workflow

1. **Baseline.** Over the last 30 days, or the last 200 runs on a quieter repository, rank the jobs by runner minutes, and count the runs per ref, per event and per actor (a bot or a pull request triggering the pipeline dozens of times). The top jobs make 80% of the minutes, plus every job on the pipeline's critical path (its longest chain of dependent jobs, read from the job graph); pull their durations and queue time over the same window. Done when each top job has a median, a p90, a queue time and the run count behind them, plus, on self-hosted or shared runners, its intrinsic and contention shares where load is readable, and the refs, events and actors with the most runs are named. How to count runner minutes, the sources (and what to do without the run API), and how to split intrinsic cost from contention: [measure.md](references/measure.md)
2. **Decompose.** Break each top job into steps, scripts, the tool's own phases and its slowest tests (from the job log). Done when the largest cost of each job, and of the critical path, has a name and a number of seconds. Timing sources and how parallel work costs: [measure.md](references/measure.md)
3. **Explain.** Find the cause of each named cost, and measure its marginal cost: locally for a local cause (the tool with and without the suspect), otherwise from the logs and the run API. Done when every cause is measured or read in a log, none assumed, and each lever under "Work that should not run" is confirmed or ruled out with the number its Confirm line names. Each cause's signs, how to confirm it, its fixes and its proof: [levers.md](references/levers.md)
4. **Diagnose with the user.** Present a table of causes with the gain in wall time per pipeline run and in runner minutes per week, each counted separately, ranked by the cost the request names (runner minutes when it names neither), what each fix needs, and how each will be proven. Discuss before writing code. Done when the user has picked the levers; a request that only asks why ends here, with the table as its answer. With no user to ask, implement only the top-ranked lever that leaves every required check proving what it proved before, take it as far as the branch run where the repository's rules allow pushing a branch (a local commit otherwise), list the others with their numbers, and say so.
5. **Research before building.** Done when each picked lever names its mechanism and the source behind it (a project file, a doc URL, an audited tool pinned to a commit), with each rejected option and one reason. Search order, the third-party audit brief, prerelease features: [tools.md](references/tools.md)
6. **Prototype, then implement.** Try the change in a throwaway form first, where it runs: locally or in a temporary worktree for a tool, on a scratch branch for workflow config, in a scratch directory for a prerelease binary. Done when the prototype moved the number in the predicted direction by more than the spread of the before runs under the same conditions, and that lever is its own commit naming the measured cause; a lever whose prototype shows no gain goes back to the user with its numbers, not into a commit.
7. **Prove it.** Run the proving stages each lever needs. Done when each lever has a before and after number from CI, with its conditions. Which stage applies to which lever, merging, and waiting on CI: [validate.md](references/validate.md)
8. **Follow up.** A question that needs time (cache growth over weeks, an upstream release) gets a follow-up before the session ends: a reminder or scheduled task where the harness has one, otherwise an issue or a TODO entry. Done when every open question has a follow-up, or its text handed to the user where creating one is not allowed, with its trigger, the last numbers and the commands to re-measure.

## Delegation

- Independent research questions run in parallel read-only research subagents (`swarm-research` when installed).
- Changes that live in another repository (shared workflows, runner config) go to an agent session in that repository when the harness can start one; otherwise list them for the user.
- Measurements you will compare stay inline, on one machine under one load.

## Principles

- **Measure before you cut.** A suspicion becomes a lever once a number backs it.
- **The cheapest job is the one that never runs.** Prefer a lever that removes work (a skip, a dropped duplicate, a narrower scope) over one that speeds it up.
- **Proof over trust.** Judge a change by evidence that the work it claims happened (a hit count, a step's conclusion, a log line), never by a green badge.
- **Fail open.** Optimization logic that cannot decide runs the full work; an API error never turns a job red.
- **Permission.** Commits, pushes, pull requests, merges, downloads and follow-ups go ahead on the user's word or the repository's rules; ask when neither says.
- **One owner per check.** Each check runs once per change, in the job that owns it; local commands keep their full chain.
- **A number's conditions.** Every figure carries its source (run id, log line, command), the load and the machine; a single run is reported as one run. "With its conditions" means all of these, wherever this skill says it.
