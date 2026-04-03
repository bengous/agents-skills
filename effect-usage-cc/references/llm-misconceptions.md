# LLM Misconceptions

APIs and terminology that LLMs commonly hallucinate or get wrong when generating
Effect code. Use this reference when reviewing Effect code for plausibility or
debugging "method not found" errors.

## Hallucinated or Outdated APIs

| Wrong (commonly hallucinated) | Correct | Notes | Source |
|---|---|---|---|
| `import { Schema } from "@effect/schema"` | `import { Schema } from "effect"` or `import * as Schema from "effect/Schema"` | `@effect/schema` was merged into the main `effect` package | `[O]` |
| `Schema.parse(...)` / `Schema.parseSync(...)` | `Schema.decodeUnknownSync(schema)(data)` | Old API; current name is `decodeUnknown` / `decodeUnknownSync` | `[O]` |
| `Schema.validate(...)` | `Schema.decodeUnknownSync(schema)(data)` | No `validate` function exists; decode IS validation | `[O]` |
| `Effect.match(effect, { onSuccess, onFailure })` | `Effect.either(effect)` + `Either.match(either, { onLeft, onRight })` or `Effect.catchTag` | `Effect.match` does not exist; use `Either.match` after `Effect.either`, or tag-based catching | `[O]` |
| `Effect.provide(layer)` as standalone | `effect.pipe(Effect.provide(layer))` or `Effect.provide(effect, layer)` | `Effect.provide` takes an effect as first arg (data-last with pipe) | `[R]` |
| `Layer.provide(layerA, layerB)` | `Layer.provide(layerA, layerB)` -- layerA depends on layerB | Argument order confusion: first arg is the layer being built, second provides its deps | `[R]` |

## Wrong Terminology

| Wrong | Correct | Notes | Source |
|---|---|---|---|
| "thread-local storage" | "fiber-local storage" via `FiberRef` | Effect uses fibers, not threads | `[O]` |
| fibers are "cancelled" | fibers are "interrupted" | Effect terminology: `Fiber.interrupt`, `InterruptedException` | `[O]` |
| "worker pool" for `concurrency` option | concurrency limiter | `Effect.all({ concurrency: 5 })` limits concurrent fibers, not a pool | `[R]` |

## Common Confusions (Not Hallucinated, But Misused)

| Pattern | Misuse | Correct Usage | Source |
|---|---|---|---|
| `Effect.cachedWithTTL` vs `Cache.make` | Confusing which to use | `cachedWithTTL` caches a single effect's result; `Cache.make` is a key-value cache with lookup | `[O]` |
| `Effect.runSync` inside `Effect.gen` | Nesting runtimes | Compose with `yield*` instead; nested `runSync` breaks structured concurrency | `[O]` |
| `Schema.Struct` field order | Assuming output preserves declaration order | JSON output order is not guaranteed by spec | `[R]` |
| all queues have back-pressure | only **bounded** queues apply back-pressure | `Queue.sliding` and `Queue.dropping` discard items instead | `[O]` |
