# Error Handling

Effect tracks errors in the type system via `Effect<Success, Error, Requirements>`. The
error channel `E` captures a union of all possible failure types, making errors discoverable
at compile time and enabling exhaustive handling `[O]`.

---

## Error Definition Approaches

### `Schema.TaggedError` — Serializable, Validated `[O]` `[T3]`

Runtime-validated error with Schema fields. Preferred for errors that cross system
boundaries (API responses, serialized logs, network transport).

```typescript
import { Schema } from "effect"

class ApiError extends Schema.TaggedError<ApiError>()(
  "ApiError",
  {
    endpoint: Schema.String,
    statusCode: Schema.Number,
    cause: Schema.optional(Schema.Defect),
  }
) {}
```

Key properties `[O]`:
- Auto-generates `_tag` discriminant for pattern matching
- Fields are Schema-validated at construction time
- Serializable — can send over network or persist to storage
- Yieldable — can be yielded directly in `Effect.gen` without `Effect.fail`
- `Schema.Defect` wraps unknown errors from external libraries safely `[O]`

### `Data.TaggedError` — Simple, No Validation `[O]` `[L]`

Lightweight error with structural equality. No runtime validation of fields.

```typescript
import { Data } from "effect"

class NotFoundError extends Data.TaggedError("NotFoundError")<{
  readonly resource: string
  readonly id: string
}> {}
```

Key properties `[O]`:
- Auto-generates `_tag` discriminant
- Built-in equality and hashing (via `Data` module)
- Yieldable — same as `Schema.TaggedErrorClass`
- No Schema overhead — faster construction
- Collects stack trace and `cause` automatically

### Decision Rule `[R]`

| Context | Use |
|---|---|
| Error crosses a boundary (API, RPC, serialization) | `Schema.TaggedErrorClass` |
| Error stays internal (within a service or module) | `Data.TaggedError` |
| Error needs runtime field validation | `Schema.TaggedErrorClass` |
| Maximum simplicity, no validation needed | `Data.TaggedError` |

---

## Error Mapping Factories `[T3]`

T3 Code uses factory functions that produce error constructors with pre-bound context.
This keeps call sites clean and centralizes error wrapping.

```typescript
// Factory: binds context, returns a function that accepts cause
const toApiError =
  (endpoint: string) =>
  (cause: unknown) =>
    new ApiError({ endpoint, statusCode: 500, cause })

// Usage in a pipeline
const fetchUser = (id: string) =>
  Effect.tryPromise({
    try: () => fetch(`/api/users/${id}`).then((r) => r.json()),
    catch: toApiError(`/api/users/${id}`),
  })
```

Pattern: `toXyzError(context) => (cause) => new XyzError(...)` `[T3]`

This separates **what went wrong** (the cause) from **where it went wrong**
(the context), producing structured errors with full diagnostic information.

---

## Error Union Patterns `[T3]`

T3 Code defines error unions as plain TypeScript types, not Schema unions:

```typescript
// Plain TS union — simple, no runtime overhead
type OrchestrationError =
  | ProviderTimeoutError
  | CheckpointError
  | SessionExpiredError
```

This keeps the error channel readable in `Effect<A, OrchestrationError, R>` without
introducing Schema union machinery for internal type tracking `[T3]`.

---

## Typed Errors vs Defects `[O]`

Effect distinguishes two failure modes:

| | Typed errors (`fail`) | Defects (`die`) |
|---|---|---|
| Created with | `Effect.fail(error)` | `Effect.die(error)` / `Effect.dieMessage(msg)` |
| Appears in type | Yes — `Effect<A, E, R>` | No — invisible in `E` |
| Recoverable | Yes — `catchTag`, `catchAll` | Only at boundaries — `catchAllDefect` |
| Purpose | Domain failures the caller can handle | Bugs, invariant violations, unrecoverable state |

### When to use each `[O]`

**Typed errors** — domain failures where recovery is meaningful:
- Validation errors, "not found", permission denied, rate limiting
- External API failures where retry or fallback is possible
- Business rule violations the caller needs to handle

**Defects** — conditions where no sensible recovery exists:
- Programming bugs (null where non-null expected)
- Invariant violations (impossible state reached)
- Exhausted retries on a critical resource (promote with `Effect.orDie`)

```typescript
// Typed error: caller can handle "not found"
const findUser = (id: string) =>
  Effect.gen(function* () {
    const row = yield* db.query("SELECT * FROM users WHERE id = $1", [id])
    if (!row) return yield* new NotFoundError({ resource: "user", id })
    return row
  })

// Defect: config missing = unrecoverable, app cannot start
const loadConfig = Effect.gen(function* () {
  const raw = yield* Effect.tryPromise(() => readFile("config.json", "utf8"))
  const parsed = JSON.parse(raw)
  if (!parsed.dbUrl) return yield* Effect.die(new Error("dbUrl missing"))
  return parsed
})
```

### Converting between typed errors and defects `[O]`

| Operator | Direction | Use case |
|---|---|---|
| `Effect.orDie` | Typed error → defect | "I've exhausted recovery, this is now fatal" |
| `Effect.orDieWith(fn)` | Typed error → custom defect | Same, with error transformation |
| `Effect.catchAllDefect(fn)` | Defect → recovery | System boundary logging/crash reporting only |

---

## Recovery Operators `[O]`

### `catchTag` — Handle one specific error

```typescript
program.pipe(
  Effect.catchTag("NotFoundError", (e) =>
    Effect.succeed({ fallback: true, resource: e.resource })
  )
)
```

Removes `NotFoundError` from the error union. Other errors remain.

### `catchTags` — Handle multiple errors at once

```typescript
program.pipe(
  Effect.catchTags({
    NotFoundError: (e) => Effect.succeed(`${e.resource} not found`),
    ApiError: (e) => Effect.succeed(`API failed: ${e.statusCode}`),
  })
)
```

### `catchAll` — Handle all typed errors

```typescript
program.pipe(
  Effect.catchAll((error) => Effect.succeed(`Recovered: ${error._tag}`))
)
```

Result has `E = never` if recovery is complete.

### `orElse` — Replace with alternative effect

```typescript
primarySource.pipe(
  Effect.orElse(() => fallbackSource)
)
```

---

## Short-Circuiting `[O]`

`Effect.gen`, `flatMap`, and `andThen` short-circuit on the first error. Remaining
computations are skipped, preventing wasted work. This is Effect's equivalent of
early return — no explicit `if (error) return` needed.

---

## Summary: Error Handling Checklist `[R]`

1. **Define errors upfront** — errors are part of the API contract, not afterthoughts
2. **Choose the right constructor** — `Schema.TaggedErrorClass` at boundaries, `Data.TaggedError` internally
3. **Use factory functions** for repetitive error wrapping (`toXyzError` pattern) `[T3]`
4. **Keep error unions as plain TS types** — no Schema union overhead for internal tracking `[T3]`
5. **Use `fail` for recoverable, `die` for bugs** — do not put invariant violations in the typed channel
6. **Prefer `catchTag`/`catchTags`** over `catchAll` — exhaustive, type-safe, self-documenting
7. **Handle defects only at system boundaries** — logging, crash reporting, graceful shutdown
