# Context Matrix

Verdict lookup for specific project contexts. Each row answers: "Should I use Effect here?"

Use the 5-level scale from SKILL.md: Required, Recommended, Optional, Discouraged, Not Appropriate.

## How to Read This Matrix

- **Verdict** is the default recommendation for that context.
- **Complexity Drivers** lists what makes Effect valuable (or not) in that context.
- **Override Condition** specifies when the verdict shifts one level up or down — encodes the non-dogmatic stance.
- Provenance markers (`[O]`, `[T3]`, `[L]`, `[R]`) tag every assertion. When in doubt, prioritize `[O]` > `[T3]` > `[L]` > `[R]`.
- The decision is always per-component, never project-wide `[R]`.

| Section | Contexts |
|---|---|
| [Backend](#backend) | API server, database, external APIs, job queues, message brokers, WebSocket |
| [CLI](#cli) | CLI tools, automation scripts, build tooling |
| [Frontend](#frontend) | React components, data fetching, state management |
| [Shared](#shared) | Domain logic, type utilities, config, schema validation |
| [Infrastructure](#infrastructure) | Logging, process management, resource lifecycle, AI pipelines |

---

## Backend

| Context | Verdict | Complexity Drivers | Override Condition |
|---|---|---|---|
| API server / HTTP handlers | Recommended | Typed error channels, middleware composition, dependency injection via Layers, request lifecycle `[O]` | Recommended -> Optional for trivial CRUD with no error recovery needs `[R]` |
| Database access layer | Recommended | Connection pooling (`Pool`), transaction scoping, typed query errors, resource safety via `acquireRelease` `[O]` | Recommended -> Optional when using an ORM that already manages connections and errors `[R]` |
| External API integrations (3rd party) | Recommended | Retry with `Schedule`, timeout, circuit breaking, typed HTTP errors, `Effect.tryPromise` at boundary `[O]` | Recommended -> Required when multiple fallback sources or rate-limit coordination needed `[R]` |
| Background job queues / workers | Required | Structured concurrency (fibers), interruption safety, resource cleanup on cancellation, retry policies, backpressure `[O]` | No downgrade — plain TS produces fragile lifecycle management `[R]` |
| Message broker consumers | Required | Consumer lifecycle, reconnection (`effect-messaging` provides auto-reconnect for AMQP/NATS), ordered processing, distributed tracing propagation `[O]` | No downgrade — consumer lifecycle without Effect requires manual state machines `[R]` |
| WebSocket handlers | Recommended | Connection lifecycle, typed push envelopes, sequence ordering, readiness gating via `Deferred` barriers `[T3]` | Recommended -> Optional for simple broadcast-only WebSocket with no ordering guarantees `[R]` |

**Evidence — Backend:**

- Effect HTTP server groups define routes with typed input/output/error schemas, validated over the wire. DDD-style API architectures (models, repositories, services, routes) map naturally to Effect's Layer system `[O]`.
- T3 Code uses typed WebSocket push envelopes with monotonic sequence numbers (`WsPushSequence`) and routes all server-to-client pushes through a single `ServerPushBus` with ordered queuing `[T3]`.
- T3 Code gates server readiness on 5 explicit `Deferred` barriers (HTTP, push bus, keybindings, terminal subs, orchestration subs) — no welcome sent until all subsystems report ready `[T3]`.
- `effect-messaging` wraps AMQP connections and NATS JetStream with auto-reconnect, seamless consumption continuation after reconnection, and distributed tracing propagation from publishers to subscribers `[O]`.
- Effect RPC is protocol-agnostic — the same typed contract works for HTTP, WebSocket, and Web Worker communication, with automatic serialization, routing, and error propagation `[O]`.
- Connection pool pattern with `Pool.makeWithTTL` manages sizing, TTL, and concurrent access; `acquireRelease` guarantees cleanup even under fiber interruption `[O]`.

```typescript
// Boundary wrapping pattern for external APIs [O] [T3]
const fetchUser = (id: string) =>
  Effect.tryPromise({
    try: () => fetch(`/api/users/${id}`),
    catch: (error) => new NetworkError({ cause: error as Error })
  })
```

---

## CLI

| Context | Verdict | Complexity Drivers | Override Condition |
|---|---|---|---|
| CLI tools with retries/cancellation | Recommended | `@effect/cli` provides typed args/flags, wizard mode, shell completions; `Schedule` for retries; fiber interruption for cancellation `[O]` | Recommended -> Required when CLI orchestrates long-running resources (connection pools, watchers) `[R]` |
| Simple automation scripts | Discouraged | Low error surface, short-lived, no resource lifecycle; Effect adds startup overhead without structural leverage `[R]` | Discouraged -> Optional if the script grows to coordinate multiple fallible I/O steps `[R]` |
| Build tooling / dev scripts | Discouraged | Typically imperative file/process operations; build tools have their own plugin systems; Effect ceremony outweighs benefit `[R]` | Discouraged -> Optional for complex build orchestration with parallel tasks and cleanup `[R]` |

**Evidence — CLI:**

- `@effect/cli` supports hierarchical commands, built-in `--help`/`--version`/`--wizard`, shell completions, and platform-specific integration via `@effect/platform-node` `[O]`.
- Effect CLI v4 simplifies flag definitions with `Flag.make` and produces compact help output; wizard mode guides users through constructing commands interactively `[O]`.
- Simple scripts lack the error surface and concurrency needs that justify Effect's lazy evaluation model. A script that reads a file and transforms it does not benefit from typed error channels or structured concurrency `[R]`.
- The boundary: if a script grows to need retries, cancellation, or resource cleanup, it has crossed into "CLI tool" territory and the verdict shifts to Recommended `[R]`.

---

## Frontend

| Context | Verdict | Complexity Drivers | Override Condition |
|---|---|---|---|
| React components / hooks | Not Appropriate | React owns the render cycle; Effect's lazy model conflicts with React's eager state updates; component logic should stay in React idioms `[O]` `[R]` | No upgrade — even complex components should use React patterns `[R]` |
| React data fetching (server) | Optional | Server-side data fetching can benefit from typed errors and retries; `ManagedRuntime` in middleware provides Effect context to server functions `[T3]` | Optional -> Recommended when server functions coordinate multiple services with error recovery `[R]` |
| Frontend state management | Not Appropriate | State libraries (Zustand, Jotai, Redux) handle reactivity; Effect adds indirection without UI benefit `[R]` | Not Appropriate -> Discouraged if using `effect-atom` for specific state synchronization needs `[R]` |

**Evidence — Frontend:**

- Official docs confirm apps run at 120fps using Effect intensively, but this refers to Effect managing backend/data logic, not React component internals `[O]`.
- T3 Code (TanStack Start) creates middleware that sets up an Effect runtime with core dependencies, then provides `runEffect` in middleware context. Effect logic is pushed to the server entry point, not scattered through components `[T3]`.
- The Micro module exists for bundle-sensitive contexts — self-contained, excludes `Layer`/`Ref`/`Queue`/`Deferred`, ideal for libraries exposing `Promise`-based APIs while using Effect internally. If any major Effect module is imported, the full runtime is bundled regardless `[O]`.
- When incrementally adopting in a fullstack app, push Effect as close to the server entry point as possible. This avoids jumping back and forth between Effect and non-Effect code `[T3]`.
- The core Effect runtime is ~25k gzipped; remaining modules tree-shake. Adopting Effect broadly has reduced final bundle size in some applications `[O]`.

---

## Shared

| Context | Verdict | Complexity Drivers | Override Condition |
|---|---|---|---|
| Pure domain logic / transforms | Not Appropriate | No side effects, no error channels, no dependencies; Effect wrapping adds ceremony the JIT would optimize away `[O]` | No upgrade — `Effect.sync(() => add(1, 2))` is guardrail #1 in SKILL.md `[O]` |
| Shared type utilities / type guards | Not Appropriate | Pure type-level or synchronous predicate logic; zero structural leverage from Effect `[R]` | No upgrade — these should remain plain TypeScript functions `[R]` |
| Config parsing / validation | Optional | `Config` module provides typed, composable config with environment variable support; useful when config has multiple sources or secret handling `[O]` | Optional -> Recommended when config requires `Redacted` for secrets or multiple fallback sources `[O]` |
| Schema validation at boundaries | Recommended | `Schema` provides encode/decode with typed errors; belongs at API input, file parsing, external data boundaries `[O]` | Recommended -> Optional for internal-only data where TypeScript interfaces suffice `[R]` |

**Evidence — Shared:**

- Official code style guidelines: avoid wrapping pure functions in Effect; keep synchronous transforms in plain TypeScript. A direct `1 + 1` is optimized by the JIT to a CPU instruction; wrapping it in Effect adds ~500x overhead for zero benefit `[O]`.
- Effect `Schema` replaces Zod at system boundaries with encode/decode, branded types (`Schema.brand`), and composable error types. Template literal schemas enforce structural constraints like URL formats `[O]`.
- `Config.layer(schema)` creates a Layer providing validated configuration from environment variables. `Redacted` securely handles sensitive config values, preventing accidental logging `[O]`.
- Internal types should use plain TypeScript interfaces; Schema validation belongs at system boundaries only — API input, file parsing, external data. This is guardrail #4 in SKILL.md `[R]`.
- Simple data transforms over collections should use plain arrays, not Effect generators — generators introduce unacceptable performance overhead for pure collection operations `[O]`.

---

## Infrastructure

| Context | Verdict | Complexity Drivers | Override Condition |
|---|---|---|---|
| Logging / observability setup | Recommended | Built-in telemetry, `@effect/opentelemetry` integration, structured logging with span propagation, no third-party logger needed `[O]` | Recommended -> Optional when project already has a mature logging setup and migration cost is high `[R]` |
| Process management / spawning | Discouraged | `child_process` is well-understood; Effect wrapping adds indirection without meaningful error channel benefit for fire-and-forget spawns `[T3]` `[R]` | Discouraged -> Optional when spawned processes need lifecycle tracking with cleanup on interruption `[R]` |
| Resource lifecycle (connections, pools) | Required | `acquireRelease` guarantees cleanup; `Pool` manages TTL, sizing, and concurrent access; `Scope` ties resource lifetime to fiber lifetime `[O]` | No downgrade — manual resource management produces leaks under interruption `[O]` |
| Agent tooling / AI pipelines | Recommended | Multi-step orchestration with retries, streaming responses via `Stream`, typed error channels for API failures, cancellation support `[R]` | Recommended -> Required when pipeline coordinates multiple LLM providers with fallback and rate limiting `[R]` |

**Evidence — Infrastructure:**

- Effect includes native OpenTelemetry integration via `@effect/opentelemetry`; tracing spans propagate automatically through Effect pipelines without manual instrumentation `[O]`.
- T3 Code wraps `fs.watch` startup in explicit `start()`/`ready` lifecycle (Effect-managed) but keeps raw process spawning in plain TypeScript via `child_process`. This demonstrates the boundary: lifecycle management benefits from Effect, but fire-and-forget spawns do not `[T3]`.
- `Pool.makeWithTTL` manages connection pools with minimum size, TTL, and concurrent access — official docs demonstrate with MySQL connections. The pool is scoped to a `Scope`, so cleanup is automatic `[O]`.
- Fault-tolerant data pipelines use `acquireUseRelease` for browser/connection lifecycle, `Schedule.exponential` with `Schedule.intersect(Schedule.recurs(3))` for bounded retries, and typed error hierarchies for selective retry decisions `[O]`.
- `@effect/ai-*` packages (Anthropic, Google, Amazon Bedrock) provide typed AI service integrations that compose with Effect's concurrency and error handling primitives `[O]`.

```typescript
// Resource lifecycle pattern [O]
const connectionPool = Pool.makeWithTTL({
  acquire: Effect.acquireRelease(
    Effect.sync(() => createConnection("mysql://...")),
    (conn) => Effect.sync(() => conn.end(() => {}))
  ),
  min: 10,
  max: 20,
  timeToLive: Duration.minutes(5)
})
```
