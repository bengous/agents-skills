---
name: software-architecture
description: >
  Choose or evolve software architecture from product needs and concrete code
  evidence. Use for architecture decisions on a new project, evaluating existing
  module boundaries or coupling, and proposing architectural improvements.
  Covers composition of modularity, vertical slices, ports/adapters, functional
  cores, domain types and deployment boundaries. Does not turn routine feature
  work, bug fixes or formatting into a whole-project architecture review.
---

# Software Architecture

Help the user choose structures that make their actual work easier to understand,
change and verify. Use patterns as hypotheses about a problem. A recognizable
architecture name is not a success criterion.

## Establish the decision

Infer the task from the request: design a new system, assess an existing one, or
implement an already chosen change. Read project instructions and relevant code
before recommending changes. Ask only for missing information that would change
the decision and cannot be learned from available artifacts. Work in the user's
language; retain the project's domain vocabulary.

- New project: establish important use cases, constraints and likely sources of
  change. Read [architecture choices](references/choices.md).
- Existing project: trace a representative flow in the affected area and identify
  a concrete obstacle or risk. Read [existing systems](references/existing.md).
- Implementation already requested: use the relevant pattern reference and stay
  within that request; do not restart an architecture-selection ceremony.

## Load only what the decision needs

| Question | Reference |
|---|---|
| What combination fits this product, team and operating environment? | [Choices](references/choices.md) |
| What is worth changing in this code, and how can it be migrated? | [Existing systems](references/existing.md) |
| Where should responsibilities, data ownership and dependencies lie? | [Modules and ports](references/modules-and-ports.md) |
| How should decisions, state transitions and external effects interact? | [Behavior and effects](references/behavior-and-effects.md) |
| Which invalid values or states can types rule out? | [Types and invariants](references/types-and-invariants.md) |
| How can a decision remain discoverable and mechanically checked? | [Verification and knowledge](references/verification-and-knowledge.md) |

Read a reference when its question is encountered, not all references at startup.
These files contain the decision essentials. Consult linked primary sources when
their detail matters; check current official documentation before using a
version-sensitive API. Do not automatically browse every link.

## Exercise judgment

- Separate observed behavior, intended requirements and your inference. Existing
  code shows what runs; it does not settle what ought to run. Resolve material
  conflicts rather than silently following either code or prose.
- For each proposed boundary or abstraction, explain the knowledge it removes
  from callers, the invariant it protects, or the independence it provides.
  Include its cost. A small boundary can be valuable without multiple adapters.
- Compare realistic alternatives, including retaining the current arrangement
  when appropriate. Explain what evidence would change your recommendation.
- Respect established architecture where it serves the task. Challenge it with
  evidence when it causes the problem; neither imitate it blindly nor rewrite it
  to match the reference examples.
- Keep the task's scope and existing authorization. A request for advice produces
  advice; an implementation request authorizes the corresponding local work.
  Do not make architectural discussion an automatic refactor or approval loop.

Give a concise recommendation with its reasons, material tradeoff and next
concrete step. Show alternatives when the choice is consequential. Use a small
diagram or table when it clarifies relationships; no mandatory report format.

## Optional specialist skills

If available, consult `unrepresentable` for detailed type design, `state-machine`
for transition modeling, or `deepen-codebase-architecture` for a requested deeper
interface audit. Read their descriptions before selecting them. Consult them
only for that subproblem; do not automatically start their entire workflows.
This skill remains usable without them. Keep project-specific decisions in the
project, not in this reusable skill.
