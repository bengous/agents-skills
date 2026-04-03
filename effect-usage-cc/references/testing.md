# Testing Effect Programs

Testing Effect code ranges from plain assertions on pure pipelines to full layer-based
testing for service-heavy architectures. Choose the lightest approach that covers your scenario.

## Approach Decision Table

| Scenario | Approach | Why |
|---|---|---|
| Pure Effect pipeline, no services | Plain vitest + `Effect.runPromise` | No Effect test infra needed `[R]` |
| Effect with service dependencies | `it.effect` + `Effect.provide` or `layer()` | Auto-provides `TestContext` `[O]` |
| Resource lifecycle (acquire/release) | `it.scoped` | Scope closes automatically, triggers finalizers `[O]` |
| Time-dependent logic | `it.effect` + `TestClock.adjust` | Deterministic time, no real delays `[O]` |
| Real clock / real random needed | `it.live` | Bypasses `TestContext`, uses live runtime `[O]` |
| Non-Effect host needing Effect services | `ManagedRuntime` + plain vitest | Explicit lifecycle in `beforeAll`/`afterAll` `[T3]` |
| Integration test, manual lifecycle | Manual `Scope` | Full control over layer acquisition/release `[T3]` |
| Queue/worker drain before assertions | Fork + `Queue.shutdown` + join | Deterministic completion, no sleep hacks `[T3]` |

## 1. @effect/vitest `[O]`

Import `it` from `@effect/vitest` instead of `vitest`:

```ts
import { Effect } from "effect"
import { describe, expect, it } from "@effect/vitest"

it.effect("processes input", () =>
  Effect.gen(function* () {
    const result = yield* myEffect("input")
    expect(result).toBe("expected")
  })
)
```

### Variants

| Function | Provides | Use when |
|---|---|---|
| `it.effect` | `TestContext` (TestClock, TestRandom) | Most tests — deterministic by default `[O]` |
| `it.live` | Live runtime (real clock/random) | Tests needing actual time or system services `[O]` |
| `it.scoped` | `TestContext` + `Scope` | Tests using `acquireRelease` or scoped resources `[O]` |
| `it.scopedLive` | Live runtime + `Scope` | Scoped resources with real clock `[O]` |
| `it.flakyTest` | Retries until success or timeout | Inherently non-deterministic tests `[O]` |

### Sharing layers with `layer()`

`layer()` creates a describe block where all tests share a pre-warmed layer `[O]`:

```ts
import { expect, layer } from "@effect/vitest"
import { Context, Effect, Layer } from "effect"

class Db extends Context.Tag("Db")<Db, { query: (sql: string) => Effect.Effect<string[]> }>() {}
const DbTest = Layer.succeed(Db, { query: () => Effect.succeed(["row1"]) })

layer(DbTest)("Database tests", (it) => {
  it.effect("runs query", () =>
    Effect.gen(function* () {
      const db = yield* Db
      const rows = yield* db.query("SELECT 1")
      expect(rows).toHaveLength(1)
    })
  )
})
```

Layer is built in `beforeAll`, torn down in `afterAll` automatically.

## 2. ManagedRuntime + plain vitest `[T3]` `[L]`

For non-Effect test hosts or incremental migration. No `TestContext` (no `TestClock`) `[R]`:

```ts
import { ManagedRuntime } from "effect"
import { it, expect, afterAll } from "vitest"

const runtime = ManagedRuntime.make(AppLayer)
afterAll(() => runtime.dispose())

it("queries database", async () => {
  const result = await runtime.runPromise(myDbQuery)
  expect(result).toHaveLength(3)
})
```

## 3. Manual Scope `[T3]`

Fine-grained lifecycle control for expensive shared resources (e.g., database connections):

```ts
import { Effect, Scope, Layer, Exit } from "effect"
import { beforeAll, afterAll } from "vitest"

let scope: Scope.CloseableScope
beforeAll(async () => {
  scope = Effect.runSync(Scope.make())
  await Effect.runPromise(Layer.buildWithScope(IntegrationLayer, scope))
})
afterAll(async () => {
  await Effect.runPromise(Scope.close(scope, Exit.void))
})
```

## 4. Deterministic queue draining `[T3]`

For producer/consumer pipelines, fork the consumer, produce items, shut down the queue
to signal completion, then join — no `sleep`-based waits:

```ts
it.effect("processes all items", () =>
  Effect.gen(function* () {
    const queue = yield* Queue.unbounded<number>()
    const consumer = yield* pipe(
      Stream.fromQueue(queue), Stream.runCollect, Effect.fork
    )
    yield* Queue.offerAll(queue, [1, 2, 3])
    yield* Queue.shutdown(queue)
    const results = yield* Fiber.join(consumer)
    expect(Chunk.toArray(results)).toEqual([1, 2, 3])
  })
)
```

## TestClock `[O]`

`it.effect` provides `TestClock` — time starts at 0, advances only via `TestClock.adjust`.
Fork the time-dependent effect, adjust, then assert:

```ts
it.effect("completes after delay", () =>
  Effect.gen(function* () {
    const fiber = yield* pipe(
      Effect.delay(Effect.succeed("done"), "10 seconds"), Effect.fork
    )
    yield* TestClock.adjust("10 seconds")
    expect(yield* Fiber.join(fiber)).toBe("done")
  })
)
```

Scheduled effects (`Effect.repeat`, `Schedule.spaced`) also respect `TestClock` `[O]`.

## Mocking services with test layers `[O]`

Create substitutes with `Layer.succeed` (static) or `Layer.sync` (stateful):

```ts
const DbTest = Layer.succeed(Db, {
  query: () => Effect.succeed(["mock-row"])
})

// Mutable state is safe in tests — JS is single-threaded [O]
const UsersTest = Layer.sync(Users, () => {
  const store = new Map<string, User>()
  return {
    create: (u: User) => Effect.sync(() => { store.set(u.id, u) }),
    find: (id: string) => Effect.sync(() => Option.fromNullable(store.get(id)))
  }
})
```

Compose with `Layer.provideMerge` to expose leaf services for arrange/assert `[O]`:

```ts
const testLayer = EventsLayer.pipe(
  Layer.provideMerge(UsersTest),
  Layer.provideMerge(DbTest)
)
```

Use `provideMerge` (not `provide`) when tests need direct access to inner services `[R]`.

## Non-Vitest test runners (bun:test, Jest) `[R]`

For test runners without `@effect/vitest`, use `Effect.runPromise` directly in
async test functions. The layer composition patterns are identical — only the
test harness differs:

```ts
import { describe, expect, test } from "bun:test"
import { Effect, Layer } from "effect"

test("queries database", async () => {
  const result = await Effect.runPromise(
    myDbQuery.pipe(Effect.provide(DbTest))
  )
  expect(result).toHaveLength(3)
})
```

No `TestContext` is available — `TestClock` and `TestRandom` require
`@effect/vitest`. For time-dependent tests, use real delays or restructure to
inject a clock service.

## Mocking @effect/platform services `[R]`

Platform services (`FileSystem`, `Path`, `CommandExecutor`) have large interfaces.
For tests that only exercise a few methods, stub partially with a type assertion:

```ts
import type { FileSystem as PlatformFileSystem } from "@effect/platform/FileSystem"
import { FileSystem } from "@effect/platform"
import { Data, Effect, Layer } from "effect"

class TestFsError extends Data.TaggedError("TestFsError")<{
  readonly message: string
}> {}

type FsStub = {
  readonly exists: (path: string) => Effect.Effect<boolean, unknown>
  readonly readFileString: (path: string) => Effect.Effect<string, unknown>
}

function makeMockFsLayer(overrides: Partial<FsStub> = {}): Layer.Layer<FileSystem.FileSystem> {
  return Layer.succeed(FileSystem.FileSystem, {
    exists: () => Effect.succeed(false),
    readFileString: () => Effect.succeed(""),
    ...overrides,
  } as unknown as PlatformFileSystem)
}
```

Key points:
- Only stub the methods your code actually calls — un-stubbed methods will
  throw at runtime, which is desirable (catches unexpected interactions) `[R]`
- Use `Data.TaggedError` in stubs, not `new Error()` — the ELS
  `globalErrorInEffectFailure` warning applies even in test code `[R]`
- `@effect/platform` also exports `FileSystem.layerNoop(partialOverrides)` but
  its defaults (`Effect.die("not implemented")`) produce defects rather than
  typed errors, which can obscure test failures `[R]`
