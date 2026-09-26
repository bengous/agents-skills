---
name: state-audit
description: |
  Recover and check the implicit state machines of EXISTING code whose behavior
  contradicts itself. Use only when at least one holds: (1) a symptom crosses two
  or more components, processes or runtimes that each keep a piece of the same
  state (the UI shows one phase, the server acts on another); (2) two or more
  symptom fixes already landed on the same flow and it still misbehaves; (3) the
  user asks to map, audit or recover the states of an existing flow. Inventories
  every piece of state and its owner, names the facts nobody owns, writes the
  invariants, and checks them on a small model (exhaustive walk, xstate/graph, or
  Quint for several processes) whose counter-example must replay the reported bug,
  then hands a single-owner target to state-machine. NOT for a bug with a local
  cause (debug it), designing a new machine (state-machine), type-level
  impossible states (unrepresentable, ts-type-audit), a UI glitch with no state
  behind it, or a whole-architecture review (software-architecture).
---

# State Audit

Code written fast, by many hands, grows its state in pieces: a status here, a
flag there, a copy in the UI, an ordering rule in a prompt. Each piece is fine
alone; together they allow combinations nobody chose. This skill finds those
pieces, proves which combinations are wrong with a counter-example, and hands a
target with one owner per fact to `state-machine`.

## Contract

- Read-only until the human approves the target. No symptom fix during the
  audit: a fix now hides the evidence the model needs.
- Every fact cites the file and the symbol it was read in. Anything else is
  written "not measured".
- The model mirrors the code as it is, not as it should be. Each model action
  names the symbol it mirrors.
- A counter-example that does not replay the reported bug means the model is
  wrong. Fix the model, never the code, until it does.

## Workflow

| Step | Do | Output |
|---|---|---|
| 1. Scope | Name the flow and the reported symptom, in the user's words. | One paragraph |
| 2. Inventory | Every piece of state the flow touches: writer, storage, lifetime, readers. Fan out a read-only subagent; check three of its claims yourself before using them. | Inventory table |
| 3. Classify | Tag each piece: source, derived, copy, volatile, prose. Flag the facts with no owner, with two writers, copies refreshed on another event than their source, dropped errors. | Classified table, red flags |
| 4. Map | Draw the pieces as parallel regions of one statechart, the ownerless fact as its own region. Replay the symptom as a sequence across runtimes. List the combinations the code allows and nothing represents. | Regions diagram, sequence, combinations |
| **Gate** | **The human reads the map and confirms the symptom is on it.** | |
| 5. Invariants | Write the rules that must hold, in plain words, each tied to the symptom or a requirement. Two to five. | Invariant list |
| 6. Verify | Model the current behavior, smallest that holds the invariants' variables. Check it with the tool that fits (below). | Counter-examples |
| 7. Target | Load `state-machine`: one phase, one owner per fact. Re-run the same invariants on the target model: all hold. | Target model, green check |
| **Gate** | **The human approves the target before any code.** | |
| 8. Consolidate | Plan slices: the owner first, then each reader switches to it, then the copies and prose rules go. Each counter-example becomes a regression test. | Slices, tests |

Details and search recipes for steps 2 to 4: `references/inventory.md`.

## Pick the verification tool

| Shape of the problem | Tool | Cost |
|---|---|---|
| One machine, finite, TypeScript without a state library | Breadth-first walk over a pure transition function, about 30 lines | none |
| The repo already runs XState v5 | `getShortestPaths` from `xstate/graph`, with `stopWhen` | already a dependency |
| Several processes, async, caches, retries, timers | Quint model, run with `bunx` or `npx`, kept out of the repo's dependencies | a spec language; Java for `quint verify` |
| Impossible states at the type level only | `unrepresentable` or `ts-type-audit` | none |

Commands, templates and pitfalls for each: `references/verify.md`.

## Artifacts

Write them where the repo keeps plans, else `plans/<date>/<slug>/`: one
`state-audit.md` (steps 1 to 5 and the results of 6), the model file, and the
target model. Text first, Mermaid for the regions and the sequence. An HTML page
is worth it only when the human asks for something interactive.

A throwaway `.ts` model inside a repo whose typecheck globs `**/*.ts` breaks that
typecheck. Name it `.mjs`, or keep it outside the tree.

## Worked example

`references/worked-example.md`: a Claude Code plugin whose planning session
kept its phase in sixteen pieces over three runtimes. The walk, `xstate/graph`
and Quint all found the reported bug in three events, and a first spurious
counter-example exposed a fact the model had left implicit.
