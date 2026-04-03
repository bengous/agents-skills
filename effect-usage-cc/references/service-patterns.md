# Service Patterns

> **Scope note** — This reference documents valid service/dependency patterns and their
> trade-offs. It does not imply that services/layers should be used everywhere. Adopting
> Effect does not mean adopting full service/layer architecture by default. These patterns
> are for cases where dependency injection, testable boundaries, or resource lifecycle
> management provide genuine structural benefit.

## When to Use Services and Layers

Use services/layers when you need `[R]`:

- **Dependency injection** — swapping implementations (prod vs test, provider A vs B)
- **Testable boundaries** — isolating side effects behind interfaces
- **Resource lifecycle** — acquiring/releasing connections, pools, file handles
- **Shared singletons** — database pools, config, loggers used across the app

Do **not** use services/layers for `[R]`:

- Pure functions or transforms (just call them directly)
- Single-use helpers with no dependencies
- Synchronous operations with one consumer (see Guardrail #3 and #7 in SKILL.md)

---

## Service Definition Approaches

Three approaches, each with distinct trade-offs.

### 1. `Context.Tag` — Foundational `[O]` `[L]`

The base primitive. Define a tag with a unique string identifier and a shape type.

```typescript
import { Context, Effect, Layer } from "effect"

class Database extends Context.Tag("Database")<
  Database,
  {
    readonly query: (sql: string) => Effect.Effect<unknown>
    readonly execute: (sql: string) => Effect.Effect<void>
  }
>() {}
```

Build a layer separately:

```typescript
const DatabaseLive = Layer.effect(
  Database,
  Effect.gen(function* () {
    const config = yield* Config
    const pool = yield* makePool(config)
    return {
      query: (sql) => Effect.tryPromise(() => pool.query(sql)),
      execute: (sql) => Effect.tryPromise(() => pool.execute(sql)),
    }
  })
)
```

**When to use**: any service shape, including primitives (`string`, `number`) that
`Effect.Service` cannot express `[O]`. Also needed when you want multiple named
layer variants (Live, Test, Staging) as static properties on the class `[L]`.

### 2. `Effect.Service` — Convenience Wrapper `[O]`

Combines service definition and default layer in one class. Introduced in v3.9.

```typescript
import { Effect } from "effect"

class UserService extends Effect.Service<UserService>()("UserService", {
  effect: Effect.gen(function* () {
    const db = yield* Database
    return {
      findById: (id: string) =>
        db.query(`SELECT * FROM users WHERE id = '${id}'`),
    }
  }),
  dependencies: [DatabaseLive],
}) {}
```

Access the auto-generated layer via `UserService.Default` (with deps) or
`UserService.DefaultWithoutDependencies` (without deps) `[O]`.

**When to use**: services whose shape is an object type (not primitives), and where
a single default implementation suffices. Reduces boilerplate compared to
`Context.Tag` + manual `Layer.effect` `[O]`.

**Limitation**: cannot be used when the service type is a primitive (`string`,
function type, etc.) — fall back to `Context.Tag` `[O]`.

### 3. `ServiceMap.Service` — Namespaced Keys `[T3]`

**Effect v4 (effect-smol) only** — not available in stable v3. Used in T3 Code for strict
separation of service identity from implementation.

```typescript
import { ServiceMap } from "effect"

class RequestContext extends ServiceMap.Service<
  RequestContext,
  { readonly userId: string }
>()("RequestContext") {}
```

**File organization convention** `[T3]`:

| Concern | Path |
|---|---|
| Service identity + shape | `<module>/Services/<Name>.ts` |
| Layer implementation | `<module>/Layers/<Name>.ts` |

**When to use**: large codebases with many services where namespaced keys and
strict file separation prevent collisions and improve discoverability `[T3]`.

### Comparison

| Criterion | `Context.Tag` | `Effect.Service` | `ServiceMap.Service` |
|---|---|---|---|
| Minimum version | v2.4+ | v3.9+ | v4 (effect-smol) |
| Boilerplate | Medium | Low | Medium |
| Primitive service types | Yes | No | No |
| Multiple layer variants | Manual (static props) | `Default` only | Manual |
| Auto default layer | No | Yes | No |
| Namespaced identity | Manual string | Manual string | Automatic |
| Adoption context | Any | Single-implementation services | Large codebases |

---

## Layer Composition Patterns

### Flat Merge with `Layer.mergeAll` `[O]`

Combine independent layers. Constructors run concurrently. Memoized by default.

```typescript
const AppLayer = Layer.mergeAll(
  ConfigLive,
  LoggerLive,
  DatabaseLive,
)
```

### Providing Dependencies with `Layer.provide` `[O]`

Wire a layer's requirements to another layer:

```typescript
const DatabaseLive = Layer.effect(Database, makeLiveDb).pipe(
  Layer.provide(ConfigLive)
)
```

### `Layer.provideMerge` Chains `[T3]`

T3 Code pattern: build the full dependency graph as a chain starting from `Layer.empty`.

```typescript
const AppLayer = Layer.empty.pipe(
  Layer.provideMerge(InfraLayer),
  Layer.provideMerge(RepositoryLayer),
  Layer.provideMerge(ServiceLayer),
)
```

Each step adds services to the accumulated context. Later steps can depend on
services from earlier steps. Reads top-to-bottom as a build order `[T3]`.

### Hierarchical Composition `[L]`

Layer tiers flowing from infrastructure to application:

```typescript
// Tier 1: Infrastructure
const InfraLive = Layer.mergeAll(
  R2ServiceLive, EmailClientLive, OpenAIServiceLive,
).pipe(Layer.provide(ConfigLive))

// Tier 2: Repositories
const RepoLive = Layer.mergeAll(
  InvoiceRepoLive, ContactRepoLive,
).pipe(Layer.provide(InfraLive))

// Tier 3: Services
const ServicesLive = Layer.mergeAll(
  InvoiceServiceLive, ContactServiceLive,
).pipe(Layer.provide(RepoLive))
```

### `Layer.unwrap` for Dynamic Selection `[T3]`

Choose a layer at runtime based on configuration or environment:

```typescript
const DatabaseLayer = Layer.unwrap(
  Effect.gen(function* () {
    const config = yield* Config
    return config.dbProvider === "postgres"
      ? PostgresLive
      : SqliteLive
  })
)
```

### `ManagedRuntime` for Framework Integration `[L]`

Bridge Effect's service world into non-Effect frameworks (Next.js, Express, etc.).
Creates a runtime singleton that builds layers once and reuses across requests.

```typescript
import { Layer, ManagedRuntime } from "effect"

const AppRuntime = ManagedRuntime.make(ServicesLive)

// In a Next.js server action or API route:
export async function handleRequest(req: Request) {
  return AppRuntime.runPromise(
    Effect.gen(function* () {
      const svc = yield* InvoiceService
      return yield* svc.findBySlug(req.params.slug)
    })
  )
}
```

`ManagedRuntime` is also an `Effect` itself — it can be provided directly
via `Effect.provide(managedRuntime)` `[O]`.

---

## Layer Memoization `[O]`

Layers are **memoized by default** (reference equality). The same layer instance
used in multiple places is allocated once. Key rules:

- Call layer-creating functions once, store the result, reuse the reference
- `Layer.fresh(layer)` bypasses memoization — creates a new instance each time
- `Layer.mergeAll` runs constructors concurrently and shares across dependents
- Local `Effect.provide` calls inside effects do **not** share — use global provision

---

## Design Principles

**Keep service interfaces clean** `[R]`: service methods should have
`Requirements = never`. Dependencies belong in the layer constructor, not the
interface.

```typescript
// Good: clean interface
{ readonly query: (sql: string) => Effect.Effect<unknown> }

// Bad: leaks dependencies
{ readonly query: (sql: string) => Effect.Effect<unknown, never, Config | Logger> }
```

**Prefer `Effect.Service` for simple cases** `[R]`: when a service has one
default implementation and an object shape, `Effect.Service` reduces boilerplate.

**Use `Context.Tag` when flexibility matters** `[R]`: multiple layer variants,
primitive shapes, or when you need full control over layer construction.

**Do not create single-use layers** `[R]`: if a service has no dependencies and
one consumer, construct it directly. See Guardrail #3 in SKILL.md.
