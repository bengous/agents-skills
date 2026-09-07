# Behavior and effects

Read when business decisions, transitions and external operations are entangled.

## Functional core, imperative shell

Make decisions from explicit inputs where practical. Keep I/O orchestration
around them. Pass time, configuration and relevant state explicitly instead of
reading hidden mutable globals inside the rule.

Conceptual example, not a complete transaction protocol:

```text
snapshot = loadOrder(orderId)
decision = decideCancellation(snapshot, currentTime)
applyDecision(decision, expectedVersion = snapshot.version)
```

The decision is testable without a database. The shell remains responsible for
concurrent updates, commits and external failure. A decision from an old snapshot
does not prove a later write is valid. Use a transaction, constraint or version
check that actually protects the invariant.

Do not create a generic effect-description language merely to make every function
pure. A direct application service may be easier to follow. Prefer the split
where it removes difficult setup or exposes meaningful rules.

## State machines

For order-dependent behavior, enumerate states and events; describe permitted
transitions, guards and effects. A short table is often enough. Include failure
paths the system can actually observe: a late success after a timeout may require
reconciliation, not a transition assuming failure. Keep modeling separate from
library selection. For the modeling workflow and implementation references, use
the `state-machine` skill when available.

## Asynchronous boundaries

Choose events or jobs when their asynchronous contract serves the task. Identify
ordering, duplicate delivery, retry behavior and who owns completion. Direct
calls remain reasonable when the caller needs an immediate result.

An idempotency key must be checked and recorded with appropriate atomicity at the
responsible receiver; a preflight `exists` check can race. A local record alone
does not make an external side effect exactly once. Use the provider's contract
or explicit reconciliation when that distinction matters.

Validate decision rules separately from integration behavior. Neither pure unit
tests nor a successful UI path alone establish transaction or retry correctness.

## Sources

Reviewed 2026-09-07; examples and concurrency cautions are engineering synthesis.

- [Bernhardt: Functional Core, Imperative Shell](https://www.destroyallsoftware.com/screencasts/catalog/functional-core-imperative-shell)
  — separate computation from I/O.
- [Joshi: Idempotent Receiver](https://martinfowler.com/articles/patterns-of-distributed-systems/idempotent-receiver.html)
  — receiver-side treatment of repeated requests.
