# Architecture choices

Read for a new system or a decision to change its overall organization.

## Start with forces, then choose dimensions

Learn the representative user operation and the constraint most likely to shape
it: business rules, response time, consistency, isolation, offline use,
deployment, hardware, compatibility or team ownership. Explore the dimensions
that matter, rather than administering a universal questionnaire. Distinguish
measured requirements from forecasts.

| Dimension | Choices and deciding evidence |
|---|---|
| Deployment | One process, a few services, or independent services. Separate when independent deployment, failure isolation, security or scaling pays for network and operational complexity. |
| Code organization | Features/vertical slices when changes span a use case; technical layers when those responsibilities form useful stable boundaries. They can be nested or combined. |
| Domain model | Direct transaction scripts for simple operations; richer domain types and behavior when rules interact and recur. DDD bounded contexts help when concepts have different meanings or owners. |
| Dependencies | Direct use of an existing platform where coupling is acceptable; ports/adapters where application policy must be isolated from external mechanisms. |
| Execution | Direct calls for straightforward coordination; jobs or events for genuine asynchronous work, fan-out or decoupled lifecycle. Name ordering and consistency costs. |
| Read/write models | A shared model unless different query and command needs justify separation. CQRS does not require separate processes, databases or event sourcing. |

These are not competing packages to pick from a single list. A modular monolith
can contain vertical slices and hexagonal modules with functional decision code.
A small program may need only functions and a clear entry point.

## Illustrative decisions

- Small administrative CRUD app: the framework's normal structure and a few
  feature modules may suffice. Do not manufacture a domain entity and repository
  interface for every table.
- Pricing application with interacting rules: isolate pricing decisions and
  define contracts for catalog and exchange-rate inputs. HTTP and batch can
  drive the same use cases.
- Offline desktop editor: emphasize document ownership, undo/redo, persistence
  and UI updates. Web-service templates would miss the important structure.
- Reusable parser library: public API, error model and compatibility are central;
  a deployment diagram is probably irrelevant.

For an uncertain choice, propose a small end-to-end implementation or focused
measurement that distinguishes alternatives. For instance, measure contention
before assuming service separation will fix it. Do not invent a scalability
requirement to justify distribution.

In an exploratory conversation, let the user settle meaningful tradeoffs before
implementation. If the user already delegated the choice, make it and explain
it. Do not add a confirmation gate merely because a pattern has a name.

## Sources

Primary-source reading reviewed 2026-09-07. The table and examples are synthesis,
not industry consensus or experimentally optimal policy.

- [Fowler: Monolith First, 2015](https://martinfowler.com/bliki/MonolithFirst.html)
  — deployment tradeoffs; the author explicitly qualifies the evidence.
- [Fowler: Bounded Context, 2014](https://martinfowler.com/bliki/BoundedContext.html)
  — differing models and their relationships.
- [Fowler: CQRS, 2011](https://martinfowler.com/bliki/CQRS.html)
  — distinct models for reads and updates, with added complexity.
