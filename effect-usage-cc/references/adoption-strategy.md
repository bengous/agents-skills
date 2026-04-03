# Adoption Strategy

Answers: "Should this project adopt Effect? How incrementally?"

---

## Should You Adopt?

Effect earns its place where it creates **structural leverage** — orchestration, error channels, resource lifecycle, retries, concurrency. If your project lacks these complexity drivers, plain TypeScript is the better choice `[R]`.

**Adopt when** your project has 2+ of these signals:
- Multiple fallible I/O boundaries (APIs, databases, queues) `[O]`
- Resource lifecycle management (connection pools, file handles, watchers) `[O]`
- Structured concurrency needs (parallel tasks, cancellation, timeouts) `[O]`
- Complex error recovery (retries, fallbacks, typed error channels) `[O]`
- Service composition with dependency injection `[O]`

**Stay with plain TS when** your project is mostly:
- Pure data transforms and business logic `[O]`
- Simple scripts with linear execution `[R]`
- UI component logic `[R]`
- Thin wrappers around well-behaved libraries `[R]`

---

## Incremental Adoption Playbook

Effect does not require an all-or-nothing rewrite. Start small, grow as complexity justifies it `[O]`.

### Phase 1: Data modules only (zero runtime cost)

Use `Option`, `Either`, `Array`, `Match` without importing the Effect runtime. These are pure data utilities that tree-shake independently `[O]`.

```typescript
import { Option, Match } from "effect"

const user = Option.fromNullable(db.get(id))
const greeting = Match.value(user).pipe(
  Match.when(Option.isSome, ({ value }) => `Hello ${value.name}`),
  Match.orElse(() => "Hello guest")
)
```

### Phase 2: Effect at a single boundary

Pick one high-complexity boundary — typically an external API integration or database layer. Wrap it with `Effect.tryPromise` and expose the result back as a `Promise` to the rest of the codebase `[T3]` `[O]`.

```typescript
// The Effect boundary — typed errors, retries
const fetchUser = (id: string) =>
  Effect.tryPromise({
    try: () => fetch(`/api/users/${id}`),
    catch: (error) => new NetworkError({ cause: error as Error })
  }).pipe(Effect.retry(Schedule.exponential("1 second")))

// The Promise boundary — plain TS consumers see Promise
export const getUser = (id: string) =>
  Effect.runPromise(fetchUser(id))
```

### Phase 3: Service layer with Layers

Once multiple boundaries use Effect, introduce `Context.Tag` services and `Layer` composition. This is where Effect's dependency injection becomes valuable `[O]`.

### Phase 4: Full Effect runtime

Set up `ManagedRuntime` at the application entry point. All services compose through Layers; `Effect.provide` happens once at the top `[O]` `[T3]`.

---

## The Boundary Rule

> Effect surface begins at the service interface, not at every I/O call `[T3]` `[R]`.

Not every `fetch`, `fs.readFile`, or `child_process.spawn` needs an Effect wrapper. The boundary is where your **service logic** begins — where you need typed errors, retries, or resource lifecycle. Below that boundary, plain TypeScript is fine `[T3]`.

**Right:** Wrap at the service method level, where error handling and composition matter.

```typescript
// Service interface — Effect boundary
class UserRepo extends Context.Tag("UserRepo")<UserRepo, {
  readonly findById: (id: string) => Effect.Effect<User, UserNotFound | DbError>
}>() {}

// Implementation — Effect.tryPromise wraps the raw call
const UserRepoLive = Layer.effect(UserRepo, Effect.gen(function* () {
  const db = yield* Database
  return {
    findById: (id) => Effect.tryPromise({
      try: () => db.query(`SELECT * FROM users WHERE id = $1`, [id]),
      catch: (e) => new DbError({ cause: e as Error })
    })
  }
}))
```

**Wrong:** Wrapping every individual I/O call in Effect when the caller is plain TypeScript and gains nothing from it `[R]`.

---

## What Stays Plain TypeScript

These categories almost never benefit from Effect. Keep them in plain TS `[T3]` `[R]`:

| Category | Why plain TS | Example |
|---|---|---|
| Process spawning | `child_process` is well-understood; Effect adds indirection without benefit for fire-and-forget spawns `[T3]` | `spawn('git', ['status'])` |
| Raw file I/O (simple) | Single reads/writes with no retry or lifecycle needs `[T3]` | `fs.readFileSync('config.json')` |
| Pure predicates | No side effects, no error channels; JIT-optimized `[O]` | `const isAdmin = (u: User) => u.role === 'admin'` |
| Data transforms | Plain arrays outperform generators; no structural leverage `[O]` | `users.filter(isActive).map(toDto)` |
| Type guards | Pure type narrowing; zero benefit from Effect `[R]` | `function isString(x: unknown): x is string` |
| Simple string/date math | Synchronous, deterministic, no failure modes `[O]` | `formatDate(new Date())` |

**Exception:** If a "plain TS" operation grows to need lifecycle management (e.g., `fs.watch` that needs cleanup on interruption), it crosses into Effect territory `[T3]`.

---

## The Wrapping Pattern: Effect.tryPromise

`Effect.tryPromise` is the canonical boundary between Promise-land and Effect-land `[O]`:

```typescript
const callExternalApi = (url: string) =>
  Effect.tryPromise({
    try: () => fetch(url).then(r => r.json()),
    catch: (error) => new ApiError({ url, cause: error as Error })
  })
```

Key properties `[O]`:
- Converts `Promise` rejection to typed `E` channel
- The `catch` function maps unknown errors to your domain error type
- Composable with `Effect.retry`, `Effect.timeout`, `Effect.catchTag`
- At the exit boundary, `Effect.runPromise` converts back to `Promise` for non-Effect consumers

---

## Micro Module for Bundle-Sensitive Contexts

When bundle size is critical (libraries, client-side code), use the `Micro` module instead of the full `Effect` runtime `[O]`.

**Micro characteristics:**
- Starts at **5kb gzipped** (vs ~25kb for full Effect runtime) `[O]`
- Self-contained — excludes `Layer`, `Ref`, `Queue`, `Deferred` `[O]`
- Compatible API surface: `Micro.tryPromise`, `Micro.gen`, `Micro.sync` `[O]`
- Ideal for libraries exposing `Promise`-based APIs with Effect internals `[O]`

**Critical constraint:** If any major Effect module (beyond `Option`, `Either`, `Array`) is imported anywhere in the bundle, the full runtime is included regardless, negating Micro's benefit `[O]`.

**Use case split:**
- Client library → `Micro` for minimal footprint, expose `Promise` APIs `[O]`
- Server consuming that library → full `Effect` with `Layer`/`Scope`/`Pool` `[O]`

---

## Boundary Identification Checklist

Use this checklist when deciding where Effect boundaries belong in a project `[R]`:

- [ ] **Does this code coordinate multiple fallible operations?** If yes, Effect boundary.
- [ ] **Does this code manage resources with acquire/release semantics?** If yes, Effect boundary.
- [ ] **Does this code need retry, timeout, or cancellation?** If yes, Effect boundary.
- [ ] **Does this code define a service interface consumed by multiple callers?** If yes, consider `Context.Tag` + `Layer`.
- [ ] **Is this code a pure function with no side effects?** If yes, keep in plain TS.
- [ ] **Is this code a one-liner wrapping a native API?** If yes, keep in plain TS.
- [ ] **Does this code run only in the browser with bundle size constraints?** If yes, consider `Micro` or keep in plain TS.

The goal is not maximum Effect coverage — it is maximum structural leverage at minimum ceremony `[R]`.
