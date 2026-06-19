# looper

A Codex skill for turning rough work into dynamic loops that Codex can execute
or convert into a `/goal`.

`looper` is user-invoked only. It is for designing, auditing, and packaging
loops, not for silently running long-running agents or risky automation.

## Invoke

```text
$looper design a loop for fixing review comments until CI passes
$looper turn this into a Codex loop
$looper make a stacked PR loop for this migration
$looper review this loop for runaway risk
$looper create a /goal from the recommended loop
$looper what can I do with loops in this repo?
```

Trigger phrases include `looper`, `design a loop`, `make this a loop`, `turn this
into a Codex loop`, `Theo loop`, `Pete loop`, `loopify`, `inner loop`, `watch
this PR`, `loop until review passes`, and `create a dynamic workflow`.

## Modes

Default design mode proposes 2-4 loop shapes and recommends one. It includes the
outer loop, possible inner loops, required context, repo surfaces, human gates,
stop conditions, pause/escalation conditions, validation evidence, failure modes,
token/cost risk, and safety fit.

Codex goal mode converts a selected loop into a compact `/goal` payload. Long
payloads should reuse `goalify` and the protected `.agents/goals/` approach
instead of inventing another goal-file mechanism.

Artifact mode writes a durable Markdown loop spec, usually under
`.agents/loops/<slug>.md`. The artifact stays simple: purpose, state model,
phases, loops, inputs, outputs, gates, retry and escalation policy, validation,
audit log expectations, and completion criteria.

Review mode audits an existing loop for infinite-loop risk, token runaway,
missing gates, unsafe side effects, vague success criteria, context drift, bad
delegation, and missing validation, then returns a revised loop.

Examples mode shows practical loops tailored to the current repo or task type:
PR feedback loops, stacked PR loops, maintenance loops, research-to-execution
loops, repo cleanup loops, and agent-skill iteration loops.

## Safety stance

The skill distinguishes read-only planning loops from loops that edit code,
create branches or PRs, watch external state, merge, deploy, install
dependencies, or change external systems.

Risky side effects require explicit approval. Production and client-sensitive
loops should pause at high-risk boundaries instead of running unattended.

## Design philosophy

Prefer loop shapes over roleplay. Use inner loops for implementation, review,
correction, validation, or research only when the work justifies them. Keep
context bounded, use deterministic commands where possible, require evidence for
validation, and stop early when a loop is not worth the cost.
