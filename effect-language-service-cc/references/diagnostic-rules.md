# Diagnostic Rules Reference

All rules from `@effect/language-service`. Organized by category.

Legend: `E` error | `W` warning | `S` suggestion | `-` off by default | `F` quick fix available

---

## Correctness

These catch real bugs — Effect code that will behave incorrectly at runtime.

| Rule | Default | Fix | Description |
|---|---|---|---|
| `floatingEffect` | E | F | Effect not yielded or assigned. The most common mistake — an Effect expression sits unused in a generator. Fix: add `yield*` or assign to a variable |
| `missingReturnYieldStar` | E | F | Missing `return yield*` for Effects with `never` success type. Without the return, TypeScript cannot narrow the type correctly |
| `missingStarInYieldEffectGen` | E | F | `yield` instead of `yield*` in a generator. Missing the `*` means the Effect is not executed |
| `classSelfMismatch` | E | F | `Self` type parameter does not match class name. Common after renaming a class |
| `missingEffectContext` | E | | Service missing from the context channel. The Effect requires a service that is not provided |
| `missingEffectError` | E | F | Error type missing from the error channel |
| `missingLayerContext` | E | | Requirements missing from a Layer's context channel |
| `effectFnImplicitAny` | E | | `noImplicitAny` for unannotated params in `Effect.fn`. Params need explicit types |
| `duplicatePackage` | W | | Multiple versions of the same Effect package loaded. Causes runtime type mismatches |
| `genericEffectServices` | W | | Services with type parameters not discriminable at runtime |
| `nonObjectEffectServiceType` | E | | `Effect.Service` type is a primitive (v3). Service types must be objects |
| `outdatedApi` | W | | APIs removed/renamed in Effect v4 |
| `outdatedEffectCodegen` | W | F | Generated code is stale. Regenerate with `codegen --force` or use the quick fix |
| `overriddenSchemaConstructor` | E | F | Overridden constructor in a Schema class |
| `unsupportedServiceAccessors` | W | F | Accessors need codegen (signature too complex for inference) |
| `anyUnknownInErrorContext` | - | | `any`/`unknown` in error or requirements channels. Off by default — enable for strict projects |

## Anti-pattern

Code that works but has structural issues — lifecycle problems, unnecessary complexity,
or patterns that will cause issues at scale.

| Rule | Default | Fix | Description |
|---|---|---|---|
| `catchUnfailableEffect` | S | | Error handling on an Effect that cannot fail. Remove the unnecessary `catchAll`/`catchTag` |
| `effectFnIife` | W | F | `Effect.fn` called as IIFE. Use `Effect.gen` instead for one-shot effects |
| `effectGenUsesAdapter` | W | | Deprecated `adapter` parameter in `Effect.gen`. Remove it |
| `effectInFailure` | W | | Effect used in the failure channel. Usually a mistake — effects belong in success |
| `effectInVoidSuccess` | W | | Nested Effect in a void success channel. The inner Effect will never execute |
| `globalErrorInEffectCatch` | W | | Catch callback returns global `Error` instead of typed Effect errors |
| `globalErrorInEffectFailure` | W | | Global `Error` type in failure channel. Use typed errors (TaggedError, Data.TaggedEnum) |
| `layerMergeAllWithDependencies` | W | F | `Layer.mergeAll` where one layer provides what another requires. Use `Layer.provide` instead |
| `leakingRequirements` | S | | Implementation services leaking into method signatures |
| `multipleEffectProvide` | W | F | Chained `Effect.provide` calls cause lifecycle issues. Merge into a single provide |
| `returnEffectInGen` | S | F | Returning an Effect in a generator produces `Effect<Effect<...>>`. Add `yield*` |
| `runEffectInsideEffect` | S | F | `Effect.run*` inside an Effect context. Use Runtime service instead |
| `schemaSyncInEffect` | S | | Schema sync methods in generators (v3). Use async variants |
| `scopeInLayerEffect` | W | F | `Layer.effect` when Scope is required. Use `Layer.scoped` instead |
| `strictEffectProvide` | - | | `Effect.provide` outside entry points. Off by default — enable for strict DI |
| `tryCatchInEffectGen` | S | | try/catch in Effect generators. Use `Effect.try` or `Effect.tryPromise` |
| `unknownInEffectCatch` | W | | Catch callback returns `unknown`. Type the error |

## Effect-native (opt-in)

All off by default. Enable progressively to migrate toward pure Effect APIs.
These flag usage of native APIs that have Effect equivalents.

| Rule | Fix | What it flags | Effect alternative |
|---|---|---|---|
| `asyncFunction` | | `async` functions | `Effect.gen` |
| `cryptoRandomUUID` | | `crypto.randomUUID()` outside generator | Effect v4 UUID service |
| `cryptoRandomUUIDInEffect` | | `crypto.randomUUID()` inside generator | Effect v4 UUID service |
| `extendsNativeError` | | Class extends native `Error` | `Schema.TaggedError` |
| `globalConsole` | | `console.*` outside Effect | `Effect.log` / `Logger` |
| `globalConsoleInEffect` | | `console.*` inside Effect | `Effect.log` / `Logger` |
| `globalDate` | | `Date.now()` / `new Date()` outside Effect | `Clock` / `DateTime` |
| `globalDateInEffect` | | `Date.now()` / `new Date()` inside Effect | `Clock` / `DateTime` |
| `globalFetch` | | `fetch` outside Effect | `HttpClient` |
| `globalFetchInEffect` | | `fetch` inside Effect | `HttpClient` |
| `globalRandom` | | `Math.random()` outside Effect | `Random` service |
| `globalRandomInEffect` | | `Math.random()` inside Effect | `Random` service |
| `globalTimers` | | `setTimeout`/`setInterval` outside Effect | Effect scheduling |
| `globalTimersInEffect` | | `setTimeout`/`setInterval` inside Effect | Effect scheduling |
| `instanceOfSchema` | F | `instanceof` for Schema types | `Schema.is` |
| `newPromise` | | `new Promise(...)` | Effect APIs |
| `nodeBuiltinImport` | | Node.js imports with Effect equivalent | `@effect/platform` |
| `preferSchemaOverJson` | | `JSON.parse`/`JSON.stringify` | Schema decode/encode |
| `processEnv` | | `process.env` outside Effect | `Effect.Config` |
| `processEnvInEffect` | | `process.env` inside Effect | `Effect.Config` |

## Style

Suggestions for cleaner, more idiomatic Effect code. No bugs — just readability.

| Rule | Default | Fix | Description |
|---|---|---|---|
| `catchAllToMapError` | S | F | `catchAll` + `Effect.fail` → simplify to `mapError` |
| `deterministicKeys` | - | F | Non-deterministic service/tag/error identifiers |
| `effectFnOpportunity` | S | F | Function returning Effect → convert to `Effect.fn` |
| `effectMapVoid` | S | F | `Effect.map(() => void 0)` → `Effect.asVoid` |
| `effectSucceedWithVoid` | S | F | `Effect.succeed(undefined)` → `Effect.void` |
| `importFromBarrel` | - | F | Barrel import → use specific path |
| `missedPipeableOpportunity` | - | F | Nested calls → `pipe()` |
| `missingEffectServiceDependency` | - | | Missing dependencies in `Effect.Service` (v3) |
| `redundantSchemaTagIdentifier` | S | F | Redundant identifier in `Schema.TaggedClass` |
| `schemaStructWithTag` | S | F | `Schema.Struct` with `_tag` → `Schema.TaggedStruct` |
| `schemaUnionOfLiterals` | - | F | Multiple `Schema.Literal` → combine (v3) |
| `serviceNotAsClass` | - | F | `ServiceMap.Service` used as variable (v4) |
| `strictBooleanExpressions` | - | | Non-boolean types in conditions |
| `unnecessaryEffectGen` | S | F | `Effect.gen` with single `return` → unwrap |
| `unnecessaryFailYieldableError` | S | F | `Effect.fail(yieldableError)` → yield directly |
| `unnecessaryPipe` | S | F | `pipe()` with no arguments |
| `unnecessaryPipeChain` | S | F | Chained `pipe` → merge into one |

---

## Enabling Rules by Project Maturity

**New project (getting started)**: Use defaults. Correctness rules catch real bugs.
Anti-pattern rules at warning help build good habits.

**Mature project (hardening)**: Enable effect-native rules at `warning` for modules that
should be pure Effect. Enable `deterministicKeys`, `strictBooleanExpressions`,
`strictEffectProvide` at warning. Turn `effectInFailure` to error.

**Migration (from v3 to v4)**: Enable `outdatedApi` at error. Run `diagnostics --severity error`
to find breaking changes.
