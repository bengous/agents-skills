# Worked example: a planning session with no single phase

A Claude Code plugin, built fast by many agents over about ten days, runs a
planning session across three runtimes: a hooks module inside Claude Code, a
local server, and a browser page. The reviewer reported:

> Claude wrote the plan while a grill (a question round) was still open. The page
> said "Drafting", as if nothing held it. After I ended the grill, Claude offered
> me to write the plan, which already existed.

## Inventory and classes

A read-only subagent listed the machines; three of its claims were checked by
hand before use (the dropped refusal, the status that ignores the hold, the only
resubmit flag). Sixteen pieces:

| Class | Pieces |
|---|---|
| source | engine mode, relay follower, turn tracker; the files on disk |
| derived | workspace kind (from the file listing), holds (first extension that answers), end-of-turn gate, status pill |
| copy | three engine caches of server facts, the page's signals, the step window |
| volatile | the pending step proposal, lost on server restart |
| prose | the ordering rules of two skill files |

Red flags: the gate's refusal is dropped by `.catch(() => …)` at the turn's end;
the status pill ignores the hold before the first version; only one of two holds
remembers to resubmit when it ends; the order grill, then propose, then plan
lives only in prose. And one fact had **no owner**: "the plan changed while the
review was held".

## Map

Five parallel regions: workspace, hold, grill phase, pending step, and the
ownerless fact drawn as its own dashed region. Replaying the symptom through
them showed that only the ownerless region moves when the plan is written, and
that nothing moves it back when the grill ends. Ten combinations the code
allowed were listed; the reported one was among them.

## Invariants

1. `heldDraftIsShown`: while a grill holds the review and the plan changed, the
   pill says it is held.
2. `noPlanStepOverAPlan`: Claude never proposes the step "write the plan" when a
   plan exists.

## Verify

The same model, about 60 lines, checked three ways:

| Tool | Invariant 1 | Invariant 2 | Time |
|---|---|---|---|
| Walk over a pure function | broken in 2 events | broken in 3 events | under 1 ms |
| `xstate/graph` | broken in 2 events | broken in 3 events | 6 ms |
| `quint verify` | broken in 2 events | broken in 3 events | about 4.5 s each |

The three-event counter-example is the reported session: open the grill, write
the plan, end the grill.

## What the tool taught

The first `quint verify` run broke invariant 2 on a different path: the
reviewer picks "plan", Claude writes it, and the rule fails. That is correct
behavior. The model had kept the proposal as pending after it was served. The
code consumes a proposal when its step is taken; the model had no such fact.
Adding it made the counter-example the reported bug. Writing the model forced
the question "when does a proposal stop being pending?", which no document
answered.

Two practical lessons: the throwaway `.ts` model broke the repo's typecheck,
which globs every `.ts` including ignored plan folders, so it moved to `.mjs`;
and `quint verify` left `_apalache-out/` at the repo root.

## Next

The target, one phase computed by the server (`drafting`, `grilling`,
`reviewing`, `approved`) with the plan-changed-while-held fact owned by it, goes
to `state-machine`, and must pass both invariants before any code.
