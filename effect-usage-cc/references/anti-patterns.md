# Anti-Patterns

Common mistakes agents make with Effect. Each entry has a bad/good code pair,
source tag, and detection heuristic for code review.

---

### 1. Wrapping Pure Functions in Effect

Pure logic the JIT already optimizes does not belong in Effect `[O]`.

```ts
// Bad
const add = (a: number, b: number) =>
  Effect.sync(() => a + b)

// Good
const add = (a: number, b: number) => a + b
```

**Why:** Effect.sync adds allocation and scheduling overhead to a CPU instruction.
Effect is for workflows with side effects, errors, or resource needs `[O]`.

**Detect:** `Effect.sync(() =>` containing only arithmetic, string ops, or pure transforms.

---

### 2. Effect.sync for Trivial Operations

If an operation cannot fail and has no side effects, keep it plain `[O]`.

```ts
// Bad
const getName = (user: User) =>
  Effect.sync(() => user.name.toUpperCase())

// Good
const getName = (user: User) => user.name.toUpperCase()
```

**Why:** Effect.sync exists to suspend side effects (I/O, mutation, throwing code).
Wrapping pure accessors wastes the abstraction `[O]`.

**Detect:** `Effect.sync` or `Effect.succeed` wrapping expressions that never throw
and touch no external state.

---

### 3. Single-Use Layers

A Layer for a service with no dependencies and one consumer is unnecessary abstraction `[R]`.

```ts
// Bad
class Config extends Context.Tag("Config")<Config, { port: number }>() {}
const ConfigLayer = Layer.succeed(Config, { port: 3000 })
// ... used in exactly one place

// Good
const config: Config = { port: 3000 }
```

**Why:** Layers earn their cost when a service has dependencies, multiple consumers,
or needs to be swapped in tests. One consumer with no deps → direct construction `[R]`.

**Detect:** `Layer.succeed` or `Layer.sync` where the tag appears in only one
`Effect.gen` block and has no test doubles.

---

### 4. Schema for Internal Types

Schema validation belongs at system boundaries, not between internal modules `[R]`.

```ts
// Bad — internal function validates its own types
const processOrder = (raw: unknown) => {
  const order = Schema.decodeUnknownSync(OrderSchema)(raw)
  return calculateTotal(order)
}

// Good — validate at boundary, pass typed data internally
const handler = (req: Request) => {
  const order = Schema.decodeUnknownSync(OrderSchema)(req.body)
  return processOrder(order) // order: Order, not unknown
}
```

**Why:** Internal code already has type guarantees from TypeScript. Runtime validation
adds overhead and obscures the real boundary `[R]`.

**Detect:** `Schema.decodeUnknown` called on parameters that are already typed
(not `unknown` or external input).

---

### 5. Nested runSync

Calling `runSync` inside an Effect pipeline breaks structured concurrency `[O]`.

```ts
// Bad
const program = Effect.gen(function* () {
  const config = Effect.runSync(loadConfig) // breaks composition
  return yield* startServer(config)
})

// Good
const program = Effect.gen(function* () {
  const config = yield* loadConfig
  return yield* startServer(config)
})
```

**Why:** `runSync` creates a new runtime boundary, discarding the parent's scope,
interruption, and context. Compose with `yield*` or `flatMap` instead `[O]`.

**Detect:** `Effect.runSync` or `Effect.runPromise` inside `Effect.gen`, `pipe`,
or any function returning `Effect`.

---

### 6. Spreading Effect "for Consistency"

Using Effect in a module just because adjacent code uses it `[R]`.

```ts
// Bad — Effect added for no structural reason
const formatDate = (d: Date) =>
  Effect.succeed(d.toISOString().slice(0, 10))

// Good — plain function, called from Effect code via yield*
const formatDate = (d: Date) => d.toISOString().slice(0, 10)
```

**Why:** Each component earns its complexity independently. Effect adds value through
error channels, resources, or concurrency — not through uniformity `[R]`.

**Detect:** `Effect.succeed` wrapping a return value in a function that has no
failure path, no resource needs, and no async operations.

---

### 7. Over-Abstracting Services

Not every function needs `Context.Tag` + `Layer` `[R]`.

```ts
// Bad
class StringUtils extends Context.Tag("StringUtils")<StringUtils, {
  capitalize: (s: string) => string
}>() {}
const StringUtilsLayer = Layer.succeed(StringUtils, {
  capitalize: (s: string) => s[0].toUpperCase() + s.slice(1)
})

// Good
const capitalize = (s: string) => s[0].toUpperCase() + s.slice(1)
```

**Why:** Services exist for stateful dependencies with lifecycle (DB connections,
HTTP clients, config). Pure transforms are just functions `[R]`.

**Detect:** `Context.Tag` for services whose implementations are stateless
pure functions with no constructor arguments.

---

### 8. Raw Fiber Manipulation

Prefer `Effect.forkScoped` over `Fiber.fork` for background work `[T3]` `[R]`.

```ts
// Bad — must manually track and interrupt
const fiber = yield* Effect.fork(pollingLoop)
// later: yield* Fiber.interrupt(fiber)

// Good — auto-interrupted when scope closes
yield* Effect.forkScoped(pollingLoop)
```

**Why:** Scoped forks participate in structured concurrency — they're automatically
interrupted when the parent scope closes. Raw fibers leak if you forget to
interrupt them `[T3]`.

**Detect:** `Effect.fork` not followed by `Fiber.join` or `Fiber.interrupt` in
the same scope. Also: `Fiber.fork` usage (prefer `Effect.fork*` variants).

---

### 9. Tacit (Point-Free) Style

Avoid passing functions directly to Effect combinators `[O]`.

```ts
// Bad — breaks type inference and stack traces
const program = pipe(data, Effect.map(transform))

// Good — explicit lambda preserves types and traces
const program = pipe(data, Effect.map((x) => transform(x)))
```

**Why:** Tacit usage with Effect combinators produces `unknown` types and opaque
stack traces. The explicit lambda costs nothing at runtime and preserves full
type inference `[O]`.

**Detect:** `Effect.map(fnName)`, `Effect.flatMap(fnName)`, `Effect.tap(fnName)`
where fnName is a reference, not an arrow function.

---

### 10. Effect in React Components

Effect pipelines do not belong inside React component bodies `[R]`.

```ts
// Bad
function UserProfile({ id }: Props) {
  const [user, setUser] = useState<User>()
  useEffect(() => {
    Effect.runPromise(fetchUser(id)).then(setUser)
  }, [id])
  return <div>{user?.name}</div>
}

// Good — isolate Effect at the service boundary
// services/user.ts
export const getUser = (id: string) => Effect.runPromise(fetchUser(id))

// UserProfile.tsx
function UserProfile({ id }: Props) {
  const [user, setUser] = useState<User>()
  useEffect(() => { getUser(id).then(setUser) }, [id])
  return <div>{user?.name}</div>
}
```

**Why:** React components are synchronous render functions. Effect's runtime,
scoping, and interruption model conflicts with React's lifecycle. Keep Effect
at the service/API boundary and expose promises to React `[R]`.

**Detect:** `Effect.run*` inside React components, hooks, or event handlers.
