# Concepts and glossary

The 5 components from the SKILL.md card, plus the canonical statechart
vocabulary from [statecharts.dev](https://statecharts.dev/).

## The 5 components in detail

### State

A distinct mode the system can occupy. At any instant the machine is in exactly
one *atomic* state (or one atomic substate per parallel region, see
`statecharts-extensions.md`).

statecharts.dev definition: *"A particular behaviour of a state machine."*
([source](https://statecharts.dev/glossary/state.html))

Examples: `idle`, `loading`, `editing`, `submitting`, `disconnected`.

### Event

A trigger for a transition. Events come from outside the machine (user input,
network response, timer fire, hardware interrupt) or from the machine itself
(internal events fired during entry / exit / action).

statecharts.dev definition: *"A trigger for a transition, typically a signal
from the outside world."*
([source](https://statecharts.dev/glossary/event.html))

### Transition

The instantaneous move from one state to another. A transition is defined by
the tuple `(source state, event, target state, optional guard, optional action)`.

statecharts.dev definition: *"The instantaneous transfer from one state to
another."*
([source](https://statecharts.dev/glossary/transition.html))

UML draws transitions as labeled arrows: `source --event[guard]/action--> target`.

### Action

Code that runs when a transition fires, when a state is entered, or when a
state is exited. Entry / exit actions belong to the state; transition actions
belong to the arrow.

statecharts.dev definition: *"Code that is executed as the state machine
performs transitions."*
([source](https://statecharts.dev/glossary/action.html))

### Guard

A boolean condition checked before a transition fires. If the guard is false,
the transition is skipped (and the event may still match a different
transition or be dropped).

statecharts.dev definition: *"A boolean check imposed on a transition to
inhibit the execution."*
([source](https://statecharts.dev/glossary/guard.html))

## Full glossary

| Term | One-line definition | Source |
|---|---|---|
| **State** | Distinct mode the machine can occupy | [statecharts.dev/state](https://statecharts.dev/glossary/state.html) |
| **Event** | Trigger from outside (or inside) that may fire a transition | [statecharts.dev/event](https://statecharts.dev/glossary/event.html) |
| **Transition** | Instantaneous move from one state to another | [statecharts.dev/transition](https://statecharts.dev/glossary/transition.html) |
| **Action** | Code that runs during transition / entry / exit | [statecharts.dev/action](https://statecharts.dev/glossary/action.html) |
| **Guard** | Boolean condition that must hold for a transition to fire | [statecharts.dev/guard](https://statecharts.dev/glossary/guard.html) |
| **Atomic state** | State with no substates (a leaf) | [statecharts.dev/atomic-state](https://statecharts.dev/glossary/atomic-state.html) |
| **Compound state** | State containing one or more substates | [statecharts.dev/compound-state](https://statecharts.dev/glossary/compound-state.html) |
| **Parallel state** | Compound state whose regions are active simultaneously | [statecharts.dev/parallel-state](https://statecharts.dev/glossary/parallel-state.html) |
| **History state** | Pseudostate that returns to the most recent active sibling | [statecharts.dev/history-state](https://statecharts.dev/glossary/history-state.html) |
| **Final state** | Helper state designating completion of its parent | [statecharts.dev/final-state](https://statecharts.dev/glossary/final-state.html) |
| **Pseudostate** | Transient state used for routing (initial, history, choice, junction, fork, join) | [statecharts.dev/pseudostate](https://statecharts.dev/glossary/pseudostate.html) |
| **Initial state** | Pseudostate marking where a (compound) state starts | [statecharts.dev/initial-state](https://statecharts.dev/glossary/initial-state.html) |
| **Internal transition** | SCXML term: stays inside a compound state without exit/entry | [W3C SCXML](https://www.w3.org/TR/scxml/) |
| **External transition** | SCXML term: exits and re-enters states (default in UML) | [W3C SCXML](https://www.w3.org/TR/scxml/) |
| **Activity** | Long-running process bound to a state's lifetime | [statecharts.dev/activity](https://statecharts.dev/glossary/activity.html) |
| **Run-to-completion (RTC)** | Each event is fully processed before the next starts; transitions are atomic | [Wikipedia: Run-to-completion](https://en.wikipedia.org/wiki/Run_to_completion_scheduling), [Samek PSiCC2 ch. 2](https://www.state-machine.com/psicc2) |

## Choosing the right granularity

A common modeling mistake is using one big state per "screen" or one tiny state
per boolean. Aim for: **one atomic state per UI rendering**. If two atomic
states would render the same UI, merge them. If one atomic state needs two
distinct renderings depending on context data, split it (or model it as a
compound state with substates).

## Sources

- [statecharts.dev (full site)](https://statecharts.dev/)
- [statecharts.dev glossary](https://statecharts.dev/glossary/)
- [Stately.ai docs: state machines and statecharts](https://stately.ai/docs/state-machines-and-statecharts)
- [W3C SCXML 1.0 Recommendation](https://www.w3.org/TR/scxml/) (defines internal vs external transitions, RTC semantics)
- David Harel, *Statecharts: A Visual Formalism for Complex Systems*, Science of Computer Programming 8, 1987 ([PDF](https://www.state-machine.com/doc/Harel87.pdf))
