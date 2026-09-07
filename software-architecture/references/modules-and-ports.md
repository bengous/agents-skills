# Modules and ports

Read when locating responsibilities, reducing coupling or isolating dependencies.

## Group by change and knowledge

Put related decisions near each other. A feature folder is useful when it lets
someone change a use case without reconstructing unrelated features. Keep shared
rules in the module that owns them; slices do not require copying pricing or
authorization rules into each handler.

Choose module names from the domain. Identify public operations, owned state and
allowed consumers. An import boundary alone does not prevent another module from
mutating the same tables. Decide how reads, writes and transactions cross the
boundary; a shared database does not imply unrestricted shared ownership.

Example: `orders/cancel-order` contains cancellation and its tests. Orders owns
allowed order transitions. Billing owns refund execution. Cancellation asks
billing for a refund through its contract instead of modifying billing tables.
The consistency and retry protocol still needs design; folders do not solve it.

## Hexagonal architecture

Use application-owned ports to describe required interactions and adapters to
translate external mechanisms. Driving adapters invoke use cases; driven
adapters satisfy application dependencies. Wire concrete implementations at an
entry point/composition root rather than inside business rules.

For example, `cancelOrder(command)` can be called by HTTP or a job. It depends on
a refund capability; a provider adapter translates the request to a payment SDK.
Business policy need not understand HTTP headers or vendor response shapes.

The point is independence of application policy from external technology, not
an obligatory number of layers. A port can be justified by testability, ownership
or vendor isolation with one production adapter. An interface mirroring every
ORM method may preserve all the coupling while adding files. Define application
capabilities at the granularity callers need.

Cost: adapters require translation and contract verification; ports can obscure
control flow. For storage-shaped CRUD, direct framework usage may be the better
tradeoff. State the coupling accepted instead of pretending it is hexagonal.

## Evaluate a boundary

Ask what a caller must know: parameters, ordering, failure modes, ownership,
consistency and performance expectations. Hide coordination when possible, but
do not conceal important effects or latency behind a misleadingly simple name.
A single `execute(options)` method need not be simpler than clear operations.

Use encapsulation and dependency checks supported by the stack. Keep legitimate
shared primitives coherent; a universal `shared` module can recreate coupling.
Enforce rules protecting real responsibilities, not identical folder structures
in fundamentally different parts of the system.

## Sources

Reviewed 2026-09-07. Examples and selection criteria are synthesis.

- [Cockburn: Hexagonal Architecture, 2005](https://alistair.cockburn.us/hexagonal-architecture)
  — the inside/outside distinction and ports/adapters.
- [Bogard: Vertical Slice Architecture, 2018](https://www.jimmybogard.com/vertical-slice-architecture/)
  — organizing around changes to use cases.
- [Bogard: architecture and AI, 2026-09-01](https://www.jimmybogard.com/vertical-slice-architecture-webinar-recording-and-whats-next/)
  — practitioner perspective, not a controlled comparison.
- [Pocock: codebase-design, current source](https://github.com/mattpocock/skills/blob/main/skills/engineering/codebase-design/SKILL.md)
  — interface depth as a lens; blanket restrictions are not requirements here.
