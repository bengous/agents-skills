# Design Workflow

Precondition: one selected candidate. If missing, ask for it or run audit mode first.

Load `workflows/deep-dive.md` before designing unless the user already supplied an equivalent deep dive.

## Independent Designs

Use 3+ independent design passes when possible. If using subagents, each prompt must include:

- why this pass exists
- selected candidate and evidence files
- current coupling and dependency category
- what complexity should be hidden
- one distinct design constraint
- output contract

Recommended design constraints:

1. Minimal interface: 1-3 entry points, strict ownership.
2. Caller-optimized interface: make the common case trivial.
3. Workflow/service interface: encode sequencing and state transitions.
4. Ports/adapters interface: for remote-owned or true external dependencies.
5. Data-model interface: when the main win is colocating schema, validation, and formatting.

Each design must output:

1. Interface signature.
2. Usage example from a real caller.
3. Complexity hidden internally.
4. Dependency strategy.
5. Test strategy and old tests to delete.
6. Migration path.
7. Tradeoffs.

Compare designs across locality, leverage, seam clarity, AI entropy, testability, and migration risk. Recommend one design or a hybrid. Be opinionated.

## Output

```markdown
## Problem Space
...

## Designs
### Design A: ...
...

## Recommendation
...

Use this design for the RFC issue, or adjust it first?
```
