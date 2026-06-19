---
name: looper
disable-model-invocation: true
description: >
  USER-INVOKED ONLY. Use when the user explicitly asks to design, audit,
  review, or convert a task into a Codex loop, dynamic workflow, nested loop,
  PR feedback loop, stacked PR loop, repo maintenance loop,
  research-to-execution loop, inner loop, Theo loop, Pete loop, loopify,
  watch this PR, loop until review passes, or create a dynamic workflow. Do
  not invoke for normal implementation, review, or planning unless the user
  asks for a loop.
license: MIT
compatibility: Designed for Codex CLI with the Agent Skills format. Can emit
  compact `/goal` payloads and defer to `goalify`/`codex-goal` for long
  protected goals.
metadata:
  version: "0.1.0"
  author: "bengous"
---

# Looper

Design dynamic Codex loops from rough intent.

## Core contract

You are designing the loop, not blindly running it.

Default output is loop options plus a recommendation. Do not execute a loop just
because it has been designed. Require explicit human approval before any loop
can edit code, create or push branches, open or update PRs, merge, deploy,
install dependencies, modify external systems, watch external state for a long
time, or spend a large token budget.

A loop is a bounded control policy around agent work:
observe state, choose the next action, act or delegate, validate evidence, then
continue, stop, pause, or escalate. The loop may contain inner loops. It should
adapt to the work rather than force a fixed persona framework.

Theo/Pete-style loop means: stop making the human manually run every prompt and
feedback pass. Design a bounded agent loop that can watch PRs, comments, CI,
issues, docs, repo state, or prior agent outputs; restart or steer work when
useful; and ask for human help only at explicit gates or ambiguity boundaries.

## Invocation gate

Use only when the user explicitly invokes `$looper`, says `looper`, or asks for
a loop with phrases such as `design a loop`, `make this a loop`, `turn this into
a Codex loop`, `Theo loop`, `Pete loop`, `loopify`, `inner loop`, `watch this
PR`, `loop until review passes`, or `create a dynamic workflow`.

Do not invoke for ordinary implementation, review, planning, prompting, or
orchestration unless the user asks to design or audit a loop.

If the user asks to execute a loop immediately, first classify side effects and
surface the approval gates. For non-trivial side effects, present the loop and
ask for approval before execution.

## Mode selection

Choose the mode from the user's request:

- Design mode is the default: propose 2-4 possible loop designs and recommend
  one.
- Codex goal mode: when the user asks for a Codex `/goal`, convert the selected
  or recommended loop into a compact goal payload.
- Artifact mode: when the loop is complex, durable, shared, or likely to exceed
  a short chat answer, write a local Markdown artifact.
- Review mode: when the user provides an existing loop or asks to audit one,
  identify risks and return a revised loop.
- Examples mode: when the user asks what loops can do, show practical examples
  tailored to the current repo or task type.

Infer reasonable details from the repo and conversation. Ask only when a missing
detail materially changes scope, side effects, safety, or acceptance criteria.

## Side-effect classes

Classify every loop by the strongest side effect it can cause:

| Class | Meaning | Approval rule |
| --- | --- | --- |
| Read-only | Research, planning, issue/PR inspection, local status checks | Safe to design; execution can proceed only within the current request |
| Local artifact | Writes `.agents/loops/*.md` or other explicitly requested planning artifacts | Allowed when artifact mode is requested or clearly useful; report path |
| Code-editing | Modifies tracked files or generated source | Requires explicit approval before execution |
| Branch/PR | Creates branches, pushes, opens PRs, comments on PRs, or updates PR bodies | Requires explicit approval; pushing/commenting needs a visible summary |
| External watcher | Polls GitHub, CI, issue trackers, docs, dependency feeds, or production-like state | Requires a finite budget, poll interval, and stop condition |
| Merge/deploy/system | Merges, deploys, changes infrastructure, installs dependencies, writes credentials, or modifies external systems | Never unattended; require a human gate for each high-risk action |

For client repos, production repos, security-sensitive code, credentials,
payments, data migrations, or public communications, prefer a pause gate over
agent discretion. Do not design unattended merge/deploy loops unless the user
explicitly asks and the loop still gates irreversible actions.

## Design mode

Given rough intent, propose 2-4 loop designs. Prefer different loop shapes, not
different roleplay personas. Each design must include:

- name;
- when to use it;
- outer loop;
- possible inner loops;
- required context;
- tools or repo surfaces it needs;
- human gates;
- stop conditions;
- pause/escalation conditions;
- validation evidence;
- failure modes;
- estimated token/cost risk: low, medium, or high;
- safety fit: personal repos, client repos, production repos, or experiments
  only.

After the options, recommend one loop and explain why in terms of risk, expected
value, available evidence, and fit to the task. If the work is too small for a
loop, recommend not using a loop and provide the smallest useful alternative.

Default design output shape:

```markdown
## Loop options

### 1. <name>
When to use: ...
Outer loop: ...
Possible inner loops: ...
Required context: ...
Tools/repo surfaces: ...
Human gates: ...
Stop conditions: ...
Pause/escalation: ...
Validation evidence: ...
Failure modes: ...
Token/cost risk: low | medium | high
Safety fit: ...

## Recommendation
<recommended loop and why>
```

Keep the output concise enough to choose from. Do not bury the recommendation in
a long taxonomy.

## Codex goal mode

When the user asks for a Codex `/goal`, convert the selected or recommended loop
into a compact payload. Do not include generic role preambles. Do not prefix the
short output with `/goal`; the human types `/goal` manually.

For short payloads, output only the raw goal payload with these sections and no
extra commentary:

```text
<objective sentence>
Context:
Constraints:
Loop structure:
Gates:
Validation:
Stop condition:
Pause condition:
Allowed side effects:
Forbidden side effects:
```

Include only sections that carry useful execution information, except when the
user explicitly asks for all fields. Keep the objective first, without an
`Objective:` label.

For payloads over 4000 characters, reuse the existing `goalify` protected goal
file approach instead of inventing a second mechanism:

1. Read `goalify/SKILL.md`.
2. Verify the `codex-goal` helper exactly as `goalify` requires.
3. Write the long payload under `.agents/goals/<slug>.md` with `codex-goal`.
4. Return only the helper result and `/goal .agents/goals/<slug>.md` usage.

Do not create, set, update, pause, resume, clear, or complete an actual Codex
goal yourself.

## Artifact mode

Use artifact mode when the loop is complex enough that chat output would become
stale or hard to execute, when the user asks for a local artifact, or when a
future agent needs a durable source of truth.

Default path:

```text
.agents/loops/<slug>.md
```

Use a lower-case safe slug derived from the loop purpose. If the repo already has
a better convention for agent artifacts, prefer that and explain the choice. Do
not create a large framework unless the user asks. A single Markdown file is the
default.

The artifact must be human-readable and agent-executable. Include:

- loop purpose;
- state model;
- phases;
- outer loop;
- inner loops;
- inputs;
- outputs;
- gates;
- retry policy;
- escalation policy;
- validation commands;
- audit log expectations;
- completion criteria.

Artifact template:

```markdown
# <Loop name>

## Purpose

## State model

## Phases

## Outer loop

## Inner loops

## Inputs

## Outputs

## Gates

## Retry policy

## Escalation policy

## Validation commands

## Audit log expectations

## Completion criteria
```

The audit log expectations should say what evidence the loop records each
iteration: timestamp, observed state, action taken, files or PRs touched,
validation command and result, human decisions, next state, and known
uncertainty. Do not claim validation passed without command output, CI status,
review state, or other explicit evidence.

Writing the artifact is not approval to run the loop. Report the path and any
side effects still requiring approval.

## Review mode

When auditing an existing loop, inspect it for:

- infinite-loop risk;
- token runaway risk;
- unclear success criteria;
- missing human gates;
- unsafe side effects;
- context drift;
- stale artifact risk;
- over-broad scope;
- bad delegation boundaries;
- too many personas;
- missing validation;
- lack of stop or pause conditions.

Return a revised loop, not only a critique. If the original loop is unsafe,
revise it to pause earlier, narrow side effects, add evidence gates, or split
outer and inner loops. Preserve the user's goal when safe.

Review output shape:

```markdown
## Audit

## Required changes

## Revised loop

## Remaining risks
```

## Examples mode

Tailor examples to the current repo or task type when possible. Avoid generic
hype. Show practical loops with gates and validation.

Useful examples:

### PR feedback loop

Outer loop watches a PR until required review threads and CI are resolved. Inner
loops classify comments, patch code, rerun targeted checks, and update the PR
summary. Human gates: ambiguous reviewer intent, high-risk architectural change,
new dependency, security-sensitive change, force-push, or merge. Stop when CI is
green and required comments are resolved. Pause if review comments conflict or
CI fails for unclear environmental reasons.

### Stacked PR loop

Outer loop breaks a larger project into dependent PRs. It opens the first PR,
runs a review/fix/validation loop, waits for merge permission, then rebases or
starts the next PR. Human gates: stack plan approval, each PR creation if the
repo is client/production, merge approval, scope changes. Stop when the stack is
merged or a dependency invalidates later slices.

### Maintenance loop

Outer loop periodically inspects dependencies, issues, CI failures, stale PRs,
or repository hygiene. It proposes bounded batches before editing. Inner loops
research release notes, apply a narrow update, run validation, and summarize
risk. Human gates: dependency major versions, production-impacting changes,
public comments, or large churn. Stop when no high-value bounded batch remains.

### Research-to-execution loop

Outer loop starts read-only. It researches possible approaches, produces 2-4
candidate loop designs, asks the human to choose or adapt one, then emits a
Codex goal or `.agents/loops/<slug>.md` artifact. Human gate: required before
any code edit or external side effect. Stop when an approved execution payload
exists or research shows execution is not justified.

### Repo cleanup loop

Outer loop finds a small cleanup theme, bounds it, edits only approved files,
runs tests, and summarizes the diff. Inner loops detect dead code, remove one
class of duplication, or normalize docs. Human gates: public API changes,
formatting churn, deleted files, dependency removal. Stop when the bounded theme
is complete or the diff grows beyond the approved size.

### Agent-skill iteration loop

Outer loop edits a skill in small iterations. Inner loops compare nearby skills,
update `SKILL.md` and `README.md`, validate frontmatter, and optionally design
evals. Human gates: adding scripts, changing installation behavior, external
side effects, eval cost. Stop when validation passes and the skill contract is
clear. Pause if assertions are unverifiable or the skill starts becoming a rigid
framework.

## Loop mechanics

Outer loops own state, sequencing, budget, gates, and final synthesis. Inner
loops handle bounded work such as implementation, review, correction,
validation, or research.

Each inner loop needs:

- input state;
- allowed actions;
- max attempts or budget;
- validation evidence;
- output state;
- escalation condition.

Use delegation only when the work is independent and evidence-backed. Read-only
research subagents must remain read-only. Give subagents short clean-context
briefs with exact paths, commands, PRs, or URLs. The main agent owns synthesis
and final decisions.

Avoid persona zoos. A skeptic, architect, reviewer, or simplifier lens is useful
only when it maps to a concrete decision or risk. Prefer loop shapes,
state transitions, and evidence gates over roleplay.

## Context and cost discipline

Every loop needs a context and cost budget. Prefer `git diff --stat`, targeted
file reads, short log tails, CI status summaries, and explicit evidence over
full transcripts or full diffs. Do not read complete long agent outputs unless
that is the only way to diagnose a blocker.

For watchers, require finite polling: maximum iterations, maximum wall-clock
budget, poll interval, and a reason to continue after each observation. Never
design a loop that watches forever without a stop or pause condition.

Stop early when the loop is not justified. A single direct action is better than
a loop for small, obvious work.

## Validation rules

Validation must be run by the agent controlling the loop or backed by explicit
external evidence such as CI status, review state, command output, or a human
approval. Do not trust another agent's claim that a gate passed without checking
the evidence.

State skipped validation explicitly with a reason and the residual risk. For
code-editing loops, include the exact validation commands when known.

## Stop, pause, and escalation

Common stop conditions:

- objective achieved with validation evidence;
- no high-value loop remains;
- max attempts, max iterations, or token budget reached;
- requested side effect is not approved;
- source state is unavailable;
- the loop would expand beyond the approved scope.

Common pause/escalation conditions:

- ambiguous or conflicting human/reviewer instructions;
- high-risk architecture, security, data, dependency, merge, deploy, or public
  communication decision;
- repeated failure in the same inner loop;
- validation failure that appears environmental or outside the approved scope;
- context drift or stale artifact detected;
- client or production repo boundary crossed.

A good default retry policy is one correction attempt for ordinary failures and
at most two attempts for the same inner loop. After that, pause with evidence and
a recommended next move.

## Quality check

Before final output, verify:

- the selected mode matches the user's request;
- the skill was explicitly invoked or explicitly requested;
- design mode includes 2-4 loop options and a recommendation;
- Codex goal mode is compact and has no generic role preamble;
- artifact mode writes a simple Markdown artifact, not a framework;
- review mode returns a revised loop;
- examples mode is practical and task-shaped;
- every proposed loop has human gates, stop conditions, pause/escalation
  conditions, validation evidence, and token/cost risk;
- dangerous side effects require explicit approval;
- production and client-sensitive loops are not unattended;
- validation evidence is explicit and not agent self-report alone.
