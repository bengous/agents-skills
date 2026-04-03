# Core Patterns

Practical patterns for writing Effect code with provenance tags.

## Effect.gen vs pipe

Both are valid; pick based on readability `[O]`.

- **`Effect.gen`** — sequential workflows, intermediate values, branching logic
- **`pipe`** — linear chains, single-expression pipelines, data-last composition

```ts
// Effect.gen: reads like async/await [O]
const program = Effect.gen(function* () {
  const user = yield* fetchUser(id)
  const order = yield* createOrder(user.id, items)
  yield* sendConfirmation(order)
  return order
})

// pipe: best for linear transforms [O]
const program = pipe(
  fetchUser(id),
  Effect.flatMap((user) => createOrder(user.id, items)),
  Effect.tap((order) => sendConfirmation(order))
)
```

Rule of thumb: more than two `flatMap` calls or any branching → use `Effect.gen` `[R]`.

## Resource Management

`acquireRelease` + `Scope` guarantee cleanup on success, failure, or interruption `[O]`.

```ts
const managedFile = Effect.acquireRelease(
  Effect.sync(() => fs.openSync(path, "r")),  // acquire
  (fd) => Effect.sync(() => fs.closeSync(fd))  // release (always runs)
)

const program = Effect.scoped(
  Effect.gen(function* () {
    const fd = yield* managedFile
    return yield* readContents(fd)
  })
)
```

- `Effect.addFinalizer((exit) => ...)` — attach cleanup to current scope `[O]`
- Finalizers run in reverse order (LIFO) `[O]`
- `Effect.scoped` closes scope and triggers all finalizers `[O]`

## Concurrency Patterns

### Queue + Effect.forever Worker

Bounded `Queue` with forked worker for back-pressured processing `[T3]`:

```ts
const worker = Effect.gen(function* () {
  const queue = yield* Queue.bounded<Job>(100)
  yield* pipe(
    Queue.take(queue),
    Effect.flatMap(processJob),
    Effect.forever
  ).pipe(Effect.forkScoped)
  return queue
})
```

Queue types: `bounded` (back-pressure), `dropping` (drops new), `sliding` (drops old),
`unbounded` (no limit) `[O]`.

### PubSub for Event Broadcasting

Every subscriber gets every message — use for fan-out `[O]`:

```ts
const program = Effect.scoped(Effect.gen(function* () {
  const pubsub = yield* PubSub.bounded<Event>(16)
  const sub1 = yield* PubSub.subscribe(pubsub)
  const sub2 = yield* PubSub.subscribe(pubsub)
  yield* PubSub.publish(pubsub, { type: "user.created", id: "123" })
  const msg1 = yield* Queue.take(sub1) // both get same event
  const msg2 = yield* Queue.take(sub2)
}))
```

Subscribe before publishing — subscribers only see subsequent messages `[O]`.

### Effect.all with Concurrency

Run independent effects with controlled parallelism `[O]`:

```ts
const results = yield* Effect.all(
  [fetchUser(id), fetchOrders(id), fetchPreferences(id)],
  { concurrency: 3 }  // or "unbounded", "inherit", undefined (sequential)
)
```

### Effect.forkScoped over Raw Fibers

Scoped forks auto-interrupt when the parent scope closes `[T3]` `[R]`:

```ts
yield* Effect.forkScoped(backgroundSync)  // preferred: auto-cleanup

// Avoid: requires manual Fiber.interrupt
const fiber = yield* Effect.fork(backgroundSync)
```

Use `Effect.fork` only when you need the `Fiber` reference for explicit join/interrupt `[O]`.

## Schema at Boundaries

Validate at system boundaries (API input, file parsing, external data). Internal types
stay as plain TypeScript interfaces `[O]` `[R]`.

```ts
const CreateUserRequest = Schema.Struct({
  name: Schema.String,
  email: Schema.String.pipe(Schema.pattern(/^[^@]+@[^@]+$/)),
  age: Schema.Number.pipe(Schema.int(), Schema.positive())
})
const parsed = Schema.decodeUnknownSync(CreateUserRequest)(requestBody)
```

### Branded IDs

`Schema.brand` creates nominally distinct ID types `[T3]`:

```ts
const UserId = Schema.String.pipe(Schema.brand("UserId"))
type UserId = typeof UserId.Type
const OrderId = Schema.String.pipe(Schema.brand("OrderId"))
type OrderId = typeof OrderId.Type
// Type error: UserId not assignable to OrderId
```

## Schedule and Retry

Schedules define repeat/retry timing `[O]`:

```ts
// Retry with exponential backoff, max 3 attempts
const policy = pipe(
  Schedule.exponential("100 millis"),
  Schedule.intersect(Schedule.recurs(3))
)
const result = yield* Effect.retry(fetchData, policy)

// Repeat every 5 seconds
yield* Effect.repeat(pollStatus, Schedule.spaced("5 seconds"))
```

Common: `Schedule.once`, `Schedule.recurs(n)`, `Schedule.spaced(duration)`,
`Schedule.exponential(base)`, `Schedule.fixed(interval)` `[O]`.

## Stream Basics

`Stream<A, E, R>` emits zero or more values — use for unbounded or chunked data `[O]`.

```ts
const s = Stream.make(1, 2, 3)                            // create
const doubled = Stream.map(s, (n) => n * 2)                // transform
const chunk = yield* Stream.runCollect(s)                  // consume → Chunk<A>
yield* Stream.runForEach(s, (n) => Console.log(n))         // consume → void
const sum = yield* Stream.runFold(s, 0, (acc, n) => acc + n)
```

Streams are pull-based and lazy — they execute only when consumed `[O]`.

### Stream Construction `[O]` `[R]`

| Constructor | Use when |
|---|---|
| `Stream.make(a, b, c)` | Known finite values `[O]` |
| `Stream.fromIterable(arr)` | Array/Set/Map to stream `[O]` |
| `Stream.fromEffect(effect)` | Single Effect → single-element stream `[O]` |
| `Stream.suspend(() => stream)` | Lazy sync stream — defers construction until consumption `[O]` |
| `Stream.unwrap(effect)` | Effect that produces a stream — use when stream setup requires Effects `[O]` |
| `Stream.fromQueue(queue)` | Consume from a Queue `[O]` |
| `Stream.asyncEffect(cb)` | Bridge callback/event APIs into streams `[O]` |

`Stream.suspend` vs `Stream.unwrap` is a common migration decision:

```ts
// Before: sync setup inside Stream.suspend
const stream = Stream.suspend(() => {
  const data = readFileSync(path, "utf8")  // sync I/O
  return Stream.fromIterable(data.split("\n"))
})

// After: Effect-based setup inside Stream.unwrap
const stream = Stream.unwrap(
  Effect.gen(function* () {
    const fs = yield* FileSystem.FileSystem
    const data = yield* fs.readFileString(path)  // Effect I/O
    return Stream.fromIterable(data.split("\n"))
  })
)
```

Rule of thumb: if the stream setup calls `yield*`, use `Stream.unwrap`.
If it's pure synchronous construction, use `Stream.suspend` `[R]`.

## Tacit Style Avoidance

Always wrap function references explicitly `[O]`:

```ts
// Bad — breaks type inference, unclear stack traces
Effect.map(fn)

// Good
Effect.map((x) => fn(x))
```

Tacit functions with optional parameters or overloads erase generics and introduce
subtle runtime bugs `[O]`.
