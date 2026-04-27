---
name: state-machine
description: |
  Use when modeling system behavior with multiple distinct modes (idle/loading/
  success/error, draft/submitted/approved, connected/disconnected/reconnecting),
  encountering boolean-flag explosion where isLoading, isError and isSuccess drift
  out of sync, designing forms, wizards, auth flows, network protocols, embedded
  controllers or workflow engines, or any time "impossible states" show up in
  code or bug reports. Provides finite-state-machine and statechart patterns with
  implementation references for 7 languages and runtimes: TypeScript discriminated
  unions, XState v5, React useReducer, Svelte 5 runes, C, Java, Rust. NOT for:
  single-boolean toggles, pure server-cache concerns better handled by TanStack
  Query or SWR, or stateless pure transformations.
---

# State Machine

Model behavior as a finite state machine to eliminate impossible states and make
every transition explicit.

## When to use

- Two or more `isXxx` booleans gate behavior and can drift out of sync
- A flow has visible idle / pending / success / error / retry phases
- A form, wizard, auth flow or protocol has order-dependent steps
- A bug report says "stuck loading", "showing both states", or "shouldn't be possible"
- An embedded controller or workflow engine is being designed

## When NOT to use

- Single boolean toggle with no side effects
- Pure server cache (use TanStack Query or SWR; their internal FSM is enough)
- Stateless pure transformations

## The 5 components

| Component | Definition |
|---|---|
| **State** | Distinct mode the system can occupy (idle, loading, success...) |
| **Event** | Trigger for a transition (click, submit, response, timeout) |
| **Transition** | In state A, on event E, go to state B (optional guard / action) |
| **Action** | Side effect run on transition, entry, or exit (fetch, log, toast) |
| **Guard** | Boolean condition that must hold for the transition to fire |

Full glossary with citations: `references/concepts.md`

## Modeling checklist

1. Enumerate states. Start with the happy path.
2. Enumerate events.
3. Draw transitions. Every state needs an exit (no dead ends).
4. List impossible state combinations and confirm they are unrepresentable.
5. Add guards for conditional transitions.
6. Define entry / exit actions per state.
7. Sketch as Mermaid `stateDiagram-v2`. See `references/visual-notation.md`.

Worked example: `references/modeling-process.md`

## Pick an implementation

| Stack | Reference |
|---|---|
| TypeScript, no lib | `references/implementations/typescript-discriminated-unions.md` |
| React, no lib | `references/implementations/react-usereducer.md` |
| Svelte 5 runes | `references/implementations/svelte5-runes.md` |
| XState v5 (cross-framework, statecharts) | `references/implementations/xstate.md` |
| C (embedded, kernel) | `references/implementations/c.md` |
| Java (backend, workflows) | `references/implementations/java.md` |
| Rust (typestate, embedded) | `references/implementations/rust.md` |

Validation manifest and rerunner: `config/validation/manifest.json` and
`scripts/validate-references.sh`.

Reach for a statechart library (XState, statig, QP, Spring StateMachine) when a
flat reducer becomes hard to audit or needs hierarchy, parallel regions, or
history. See `references/statecharts-extensions.md`.

## Common patterns

Form, data fetching with retry, auth, multi-step wizard, toggle with optimistic
UI, network protocol handshake. Diagrams and implementation sketches in
`references/patterns.md`.

## Don't

`references/anti-patterns.md` covers boolean explosion, isLoading flags, syncing
derivable state, dead-end states, and switch-FSM without a typed table.

## Further reading

`references/further-reading.md` curates Harel 1987, the W3C SCXML 1.0
recommendation, statecharts.dev, Samek's PSiCC2, Khourshid talks, and Wuyts'
state-machine blog series.
