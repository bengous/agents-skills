# Strategy — Choose a Hook Approach

Select the right hook implementation strategy based on the project's context and requirements.

## Decision tree

```
Do you need hooks at all?
├─ Just one simple guard or notification → Standalone bash
├─ Medium-complexity logic, Python available → Standalone python
├─ Repo uses Bun/TS, want type safety without framework → Standalone bun
└─ Multiple hooks, shared logic, want testing + response safety → Opinionated typed runtime
```

## Strategy comparison

| Factor | Bash | Python | Bun standalone | Opinionated runtime |
|--------|------|--------|----------------|---------------------|
| **Setup cost** | None | None | `bun init` | Install typed runtime |
| **Dependencies** | bash | python3 | bun | bun + effect |
| **Type safety** | None | None | Manual types | Full event/response types |
| **Testing** | Manual pipe | Manual pipe | bun:test | Test harness + factories |
| **Response builders** | Manual JSON/stderr | Manual JSON/stderr | Manual JSON/stderr | Type-safe builders |
| **Best for** | Simple guards, notifications | JSON processing, file ops | Typed standalone logic | Complex multi-hook systems |
| **Portability** | Highest | High | Bun required | Bun required |

## When to use each

### Standalone bash
- Block a specific command pattern (e.g., `rm -rf /`)
- Send a desktop notification on Stop
- Simple stdin field extraction with `jq`
- No runtime dependencies beyond bash

### Standalone python
- Python is the project's primary language
- Hook needs file system operations or complex string processing
- Team is more comfortable with Python than TypeScript
- `json` module in stdlib handles stdin parsing

### Standalone bun
- Project already uses Bun or TypeScript
- Want type annotations without a framework
- Single hook file that reads stdin JSON and exits with a code
- Comfortable writing stdout JSON manually

### Opinionated typed runtime
- Multiple hooks sharing validation logic
- Want compile-time guarantees on event input shapes
- Want type-safe response builders (no manual JSON construction)
- Need test harness with payload factories
- Building a hook system that will grow over time

## Next steps

- Ready to set up infrastructure: [bootstrap](../bootstrap/index.md)
- Ready to write a hook: [authoring](../authoring/index.md)
- Want to see examples first: [examples](../../examples/index.md)
