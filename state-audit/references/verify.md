# Verify the model

Step 6 of the audit, and the check of the target in step 7.

## Write the model

- Model the code as it is. One action per real event, a comment naming the
  symbol it mirrors.
- Keep only the variables the invariants read, plus what the actions need to
  decide. Derived values are functions, never variables.
- Model a dropped error as an action that changes nothing: that is what the code
  does, and the tool shows it as a repeated state.
- Stay under about 80 lines. A bigger model is a second audit.

The first counter-example must replay the reported bug. If it shows another
path, read it before believing it: a spurious counter-example means the model
lacks a fact the code holds. Name that fact, add it, run again.

## A. Breadth-first walk, no dependency

For one finite machine in TypeScript. Breadth-first order makes the first
counter-example a shortest one.

```ts
type Step<S, E> = { event: E; state: S };
type Reached<S, E> = { state: S; path: Step<S, E>[] };

export function walk<S, E>(
  init: S,
  events: readonly E[],
  next: (state: S, event: E) => S | null, // null: the event does not apply here
  maxDepth = 12,
  key: (state: S) => string = (s) => JSON.stringify(s),
): Reached<S, E>[] {
  const seen = new Map<string, Reached<S, E>>([[key(init), { state: init, path: [] }]]);
  let frontier: Reached<S, E>[] = [{ state: init, path: [] }];
  for (let depth = 0; depth < maxDepth && frontier.length > 0; depth++) {
    const following: Reached<S, E>[] = [];
    for (const { state, path } of frontier) {
      for (const event of events) {
        const after = next(state, event);
        if (after === null || seen.has(key(after))) continue;
        const reached = { state: after, path: [...path, { event, state: after }] };
        seen.set(key(after), reached);
        following.push(reached);
      }
    }
    frontier = following;
  }
  return [...seen.values()];
}

export function check<S, E>(reached: Reached<S, E>[], rules: Record<string, (s: S) => boolean>) {
  return Object.entries(rules).map(([name, holds]) => ({
    name,
    counterExample: reached.find((r) => !holds(r.state)) ?? null,
  }));
}
```

`maxDepth` bounds a counter or any unbounded value. Once the model holds, the
same `next` and rules become a test in the repo's runner.

## B. xstate/graph

When the code already runs XState v5, check the real machine, not a copy.

```ts
import { getShortestPaths } from "xstate/graph";

const paths = getShortestPaths(machine, {
  events: [{ type: "openGrill" }, { type: "writePlan" }, { type: "endGrill" }],
  stopWhen: (s) => s.context.versions >= 2, // unbounded context: without it the walk never ends
});
const broken = paths
  .filter((p) => !holds(p.state))
  .toSorted((a, b) => a.steps.length - b.steps.length)[0];
```

- v5 options: `events`, `filterEvents`, `limit`, `fromState`, `stopWhen`,
  `toState`, `serializeState`, `serializeEvent`. The v4 `filter` option is gone:
  TypeScript rejects it (TS2353), plain JS ignores it and the walk may never end.
- XState has no invariant: the loop above is the check.
- Each path's `steps` lists `{ state, event }`, the first being `xstate.init`.
- It walks one machine. Actors the machine spawns are not explored: not
  measured, so model several processes with Quint.

## C. Quint

For several processes, interleavings, retries and timers. Quint models state
variables and actions; `quint run` simulates random traces, `quint verify`
explores every trace up to `--max-steps` and returns a shortest counter-example.

```quint
module session {
  var hold: str            // "free" | "grill"
  var plan: str            // "none" | "draft" | "recorded"
  var proposed: str        // "none" | "plan" | "other"

  action init = all { hold' = "free", plan' = "none", proposed' = "none" }

  // mirrors <symbol>: what the code does on this event
  action openGrill = all { hold == "free", hold' = "grill", plan' = plan, proposed' = "none" }
  action writePlan = all { plan' = "draft", hold' = hold, proposed' = "none" }
  action endGrill = all {
    hold == "grill", hold' = "free", plan' = plan,
    proposed' = if (plan == "recorded") "other" else "plan",
  }

  action step = any { openGrill, writePlan, endGrill }

  // a rule that must hold in every reachable state
  val noPlanStepOverAPlan = proposed == "plan" implies plan == "none"
}
```

Assign every variable in every action, `x' = x` for the ones it keeps: `quint
run` reports no error for an action that skips one (measured on 0.32.0).
Commands, from the project root:

```bash
bunx @informalsystems/quint run model.qnt --invariant=noPlanStepOverAPlan --max-steps=8 --mbt
bunx @informalsystems/quint verify model.qnt --invariant=noPlanStepOverAPlan --max-steps=8
```

- `run` is the simulator: milliseconds, not exhaustive, a counter-example that
  may be longer than needed. `--mbt` names the action taken at each state.
- `--seed` alone sets `--max-samples` to 1; pass both.
- `verify` uses Apalache, needs Java 17 or later, and returns a shortest
  counter-example. It writes `_apalache-out/` in the current directory, with no
  option to move it: delete it after, or run from a temp directory with an
  absolute path to the model.
- The first `run` downloads a Rust evaluator to `~/.quint/`; the first `verify`
  downloads Apalache there too.
- Model and code are separate: a green model proves the design, not the code.
  Traces from `quint run --mbt --n-traces=<n> --out-itf=<file>` can drive the
  code in tests; the Quint Connect library does that for Rust, other languages
  need a driver written for the repo.

Docs: https://quint.sh/docs/what-does-quint-do,
https://quint.sh/docs/simulator, https://quint.sh/docs/model-based-testing.

## After the check

- Each counter-example becomes a regression test: the same events replayed on
  the real code, asserting the invariant.
- The target model from `state-machine` passes the same invariants before any
  code is written.
