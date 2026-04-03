---
name: effect-usage-cc
description: >
  Decision support for Effect (effect-ts) in TypeScript projects. Determines when to use Effect,
  which patterns to prefer, and where plain TypeScript is the better choice. Invoke when reviewing
  architecture, writing Effect code, evaluating adoption, or questioning whether Effect fits a task.
  NOT for: plain TypeScript without Effect, React component logic, simple Bun scripts with no
  error recovery.
model: opus
allowed-tools:
  - Read
  - Glob
  - Grep
  - WebFetch
  - WebSearch
  - mcp__Context7__resolve-library-id
  - mcp__Context7__query-docs
  - mcp__exa__web_search_exa
  - mcp__exa__get_code_context_exa
---

# Effect Usage

## This Skill Is Guidance

This skill helps agents make better-informed decisions about Effect usage. It is not a governance
framework or rigid decision engine. Agents should use it to navigate trade-offs and find relevant
patterns — not to replace architectural reasoning. Context, project requirements, and user intent
take precedence over any guidance here.

## Core Doctrine

Effect is a structured description of a workflow — lazy, immutable, and composable `[O]`. It models
interactions rather than performing them; execution is deferred to a runtime. This makes Effect
programs testable, interruptible, and resource-safe by construction.

Use Effect where it creates **structural leverage**: orchestration, error channels, resource
lifecycle, retries, concurrency. Do not use Effect where it only adds **ceremony**: pure logic,
simple transforms, trivial I/O wrappers.

The decision should be per-component, not project-wide `[R]`.

## Almost Never Use Effect For

These cases almost never benefit from Effect. Stay in plain TypeScript:

- Pure helper functions — formatters, mappers, type guards, predicates
- Trivial synchronous logic — string formatting, date math, object mapping
- Component-local UI logic — React state, event handlers, rendering
- Simple data transforms over collections — use plain arrays, not generators `[O]`
- One-liner wrappers around native APIs with no error recovery needs
- Basic arithmetic or operations the JIT optimizes to CPU instructions `[O]`

## Adoption Decision Table

Quick verdicts. Load `references/context-matrix.md` for the full table with override conditions.

| Context | Verdict |
|---|---|
| API server / HTTP handlers | Recommended |
| CLI tools with retries/cancellation | Recommended |
| Job queues / workers / pipelines | Required |
| Database access layer | Recommended |
| FileSystem / subprocess in Effect services | Recommended |
| External API integrations | Recommended |
| React components / hooks | Not Appropriate |
| Pure domain logic / transforms | Not Appropriate |
| Simple scripts (no error recovery) | Discouraged |
| Config parsing / validation | Optional |
| Shared type utilities | Not Appropriate |

## Critical Guardrails

Common mistakes agents make with Effect. Numbered for reference in reviews.

1. **Wrapping pure functions in Effect** — `Effect.sync(() => add(1, 2))` adds ceremony to code
   the JIT already optimizes. Keep pure logic in plain TypeScript `[O]`.
2. **Effect.sync for trivial operations** — If the operation cannot fail and has no side effects,
   it does not belong in an Effect pipeline.
3. **Single-use Layers** — Creating a Layer for a service with no dependencies and one consumer
   wastes abstraction. Use direct construction `[R]`.
4. **Schema for internal types** — Schema validation belongs at system boundaries (API input,
   file parsing, external data). Internal types should use plain TypeScript interfaces `[R]`.
5. **Nested runSync** — Calling `runSync` inside an Effect pipeline breaks structured concurrency
   and resource safety. Compose with `Effect.gen` or `pipe` instead `[O]`.
6. **Spreading Effect "for consistency"** — Using Effect in a module just because adjacent modules
   use it. Each component earns its complexity independently `[R]`.
7. **Over-abstracting services** — Not every function needs to be a service. A synchronous
   transform does not need `Context.Tag` + `Layer` just because it is called from Effect code `[R]`.
8. **Raw Fiber manipulation** — Prefer `Effect.forkScoped` over direct `Fiber.fork`. Scoped forks
   are interrupted automatically when the parent scope closes `[T3]` `[R]`.
9. **Tacit (point-free) style** — Avoid `Effect.map(fn)`. Use `Effect.map((x) => fn(x))`.
   Tacit usage breaks type inference and produces unclear stack traces `[O]`.
10. **Effect in React components** — Effect pipelines do not belong inside component bodies,
    hooks, or event handlers. Keep Effect at the service boundary and expose promises to
    React `[R]`.
11. **`require()` to avoid `nodeBuiltinImport`** — The ELS `nodeBuiltinImport` warning flags
    direct `import ... from "node:*"` to nudge toward `@effect/platform` services. Silencing
    it with `require("node:fs") as typeof import("node:fs")` trades a useful signal for
    boilerplate. Instead: use `@effect/platform` in Effect service code (the warning's intended
    path), or use ESM imports with per-file `@effect-diagnostics-next-line nodeBuiltinImport:off`
    in non-Effect code (scripts, pure layers, test fixtures) `[R]`.

## Fetching Documentation

This skill captures patterns and decisions — not exhaustive API docs. When you need current API
details, type signatures, or module documentation:

| Question domain | Tool | Method |
|---|---|---|
| Effect API, types, modules | `mcp__Context7__query-docs` | Resolve `effect` library ID, then query |
| @effect/* package APIs | `mcp__Context7__query-docs` | Resolve specific package ID |
| Code patterns, integration examples | `mcp__exa__get_code_context_exa` | Natural language query |
| General research, blog posts | `mcp__exa__web_search_exa` | Semantic search |
| Official docs (fallback) | `WebFetch` | Fetch from effect.website |

Key official pages:
- Code style guidelines: `https://effect.website/docs/code-style/guidelines/`
- Myths (when NOT to use): `https://effect.website/docs/additional-resources/myths/`
- Error management: `https://effect.website/docs/error-management/expected-errors/`
- Layers: `https://effect.website/docs/requirements-management/layers/`
- Scope / resources: `https://effect.website/docs/resource-management/scope/`

Trust this skill for architectural decisions. Fetch docs for API specifics, type signatures,
or when the skill's guidance is insufficient for the task at hand.

## For Consuming Agents

### Verdict Scale

Use exactly these five levels when assessing Effect applicability:

| Verdict | Meaning |
|---|---|
| **Required** | Effect is essential; plain TypeScript would produce fragile or unmanageable code |
| **Recommended** | Effect provides significant structural benefit; plain TS is possible but worse |
| **Optional** | Either approach works; Effect adds marginal value |
| **Discouraged** | Effect adds ceremony without proportional benefit |
| **Not Appropriate** | Effect would actively harm readability, maintainability, or performance |

### Provenance Markers

Every assertion in reference files is tagged with its source. Consuming agents should
propagate these markers when citing skill content in reviews.

## Navigating References

Start from this root — the decision table and guardrails handle most cases. Load a reference
when the root guidance is insufficient for the task. How deep to go depends on task complexity,
architectural ambiguity, and whether you need a verdict, a specific pattern, or a deeper
investigation. Loading multiple references is fine when the task spans concerns (e.g., service
patterns + error handling for a full service design). The skill is designed for navigating to
the right depth, not for reading everything upfront.

## Reference Docs

| File | Answers |
|---|---|
| `references/context-matrix.md` | What verdict for this context/scenario? |
| `references/adoption-strategy.md` | Should this project adopt Effect? How incrementally? |
| `references/core-patterns.md` | How to use generators, pipes, resources, streams, schema? |
| `references/anti-patterns.md` | Is this code an anti-pattern? How to fix it? |
| `references/service-patterns.md` | How to structure services, layers, dependency injection? |
| `references/error-handling.md` | How to handle errors, typed channels, defects? |
| `references/testing.md` | How to test Effect programs and services? |
| `references/llm-misconceptions.md` | Is this API real or hallucinated? What's the correct name? |

## Knowledge Provenance

Every assertion in reference files is tagged with one of four source tiers:

| Marker | Source | Authority |
|---|---|---|
| `[O]` | Official Effect documentation | Authoritative — treat as ground truth |
| `[T3]` | T3 Code (pingdotgg/t3code) | Strong production signal — not universal law |
| `[L]` | Local codebases (Recall, Vex, Moment) | Informative — may reflect learning-stage patterns |
| `[R]` | Skill author recommendation | Architectural opinion — weigh against project context |

When sources conflict, priority is `[O]` > `[T3]` > `[L]` > `[R]`.

!`echo "## Live Project - Effect signals"`
!`~/.claude/skills/effect-usage-cc/scripts/probe-effect-posture.sh`
