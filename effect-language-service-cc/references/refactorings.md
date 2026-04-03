# Refactorings Reference

IDE code actions available via the "Refactor..." menu. These are not CLI commands — they
run through the TypeScript language service plugin in your editor.

---

## Async → Effect Conversion

Four variants for converting async functions. Choose based on error handling needs.

| Action | Result | When to use |
|---|---|---|
| `asyncAwaitToGen` | `Effect.gen` with `Effect.tryPromise` | Default — wraps all awaits uniformly |
| `asyncAwaitToGenTryPromise` | `Effect.gen` with tagged errors per `await` | Each await gets its own error type — better for functions with multiple failure points |
| `asyncAwaitToFn` | `Effect.fn` with `Effect.tryPromise` | When the function should be traced (appears in spans) |
| `asyncAwaitToFnTryPromise` | `Effect.fn` with tagged errors | Traced + per-await error typing |

**Example** (`asyncAwaitToGen`):

```ts
// Before
async function getUser(id: string) {
  const res = await fetch(`/users/${id}`);
  return res.json();
}

// After
const getUser = (id: string) =>
  Effect.gen(function* () {
    const res = yield* Effect.tryPromise(() => fetch(`/users/${id}`));
    return yield* Effect.tryPromise(() => res.json());
  });
```

**Choosing `Gen` vs `Fn`**: Use `Effect.fn` when the function represents a logical
operation that should appear in traces/spans (service methods, API handlers, jobs).
Use `Effect.gen` for internal composition that doesn't need its own span.

## Gen ↔ Fn Conversion

| Action | Trigger | Result |
|---|---|---|
| `effectGenToFn` | Function returning `Effect.gen` | Converts to `Effect.fn` |

The reverse (fn→gen) is a manual edit: remove the `Effect.fn` wrapper and its span name.

## Layer Magic

| Action | Trigger | Result |
|---|---|---|
| `layerMagic` | `Layer.mergeAll(...)` or any layer expression | Automatic composition via dependency analysis |

Analyzes `Provides` and `Requires` of each layer and generates the optimal
`Layer.provide` / `Layer.provideMerge` order. This is the IDE equivalent of the
`layerinfo` CLI command.

## Pipe Style

| Action | Trigger | Result |
|---|---|---|
| `togglePipeStyle` | `x.pipe(f)` or `pipe(x, f)` | Toggles between method pipe and function pipe |
| `pipeableToDatafirst` | `pipe(x, f)` | Rewrites to data-first style |
| `wrapWithPipe` | Nested function calls | Wraps in `pipe()` |

## Schema Generation

| Action | Trigger | Result |
|---|---|---|
| `structuralTypeToSchema` | Interface or type alias | Structural Schema |
| `typeToEffectSchema` | Type alias | Effect Schema class |
| `typeToEffectSchemaClass` | Interface | Effect Schema class |
| `makeSchemaOpaque` | Schema declaration | Opaque Schema |
| `makeSchemaOpaqueWithNs` | Schema declaration | Opaque Schema with namespace |

Typically used at system boundaries (API input, file parsing, external data).

## Service Accessors

| Action | Trigger | Result |
|---|---|---|
| `writeTagClassAccessors` | `Effect.Service`, `Context.Tag`, `Effect.Tag` | Static accessor methods |
| `effectServiceToClassWithLayer` | `Effect.Service` (v3) | `Context.Tag` + static Layer |

`writeTagClassAccessors` generates the same code as the `// @effect-codegens accessors`
directive. Use the refactoring for one-off generation, use the directive for keeping
accessors in sync automatically.

## Miscellaneous

| Action | Trigger | Result |
|---|---|---|
| `removeUnnecessaryEffectGen` | `Effect.gen` with single yield | Removes the wrapper |
| `wrapWithEffectGen` | Effect expression | Wraps in `Effect.gen` |
| `toggleReturnTypeAnnotation` | Function declaration | Adds/removes return type annotation |
| `functionToArrow` | Function declaration | Converts to arrow function |
| `toggleLazyConst` | Const declaration | Toggles lazy/eager evaluation |
| `toggleTypeAnnotation` | Variable declaration | Toggles type annotation |

## Rename Propagation

Not a refactoring action per se, but the rename feature propagates automatically:
renaming a class updates the string identifier in `TaggedError`, `TaggedClass`,
`Schema.TaggedClass`, etc.

```ts
// Renaming "MyError" → "AppError" also updates:
class MyError extends Schema.TaggedError<MyError>()("MyError", { ... })
//                                                   ^^^^^^^^^ updated
```
