# Codegen Directives Reference

Comments placed before declarations that trigger automatic code generation.
The `codegen` CLI command processes them in batch; the IDE plugin applies them in real-time.

---

## `// @effect-codegens annotate`

Adds explicit type annotations to exported `const` declarations.

```ts
// @effect-codegens annotate
export const foo = Effect.succeed(1)

// Generated:
export const foo: Effect.Effect<number> = Effect.succeed(1)
```

**When to use**: Exported effects that consumers depend on. Explicit types improve
IDE performance (TypeScript doesn't need to re-infer) and make the API surface clear.

Applies to: `const`, lists of `const`, `let`/`var`.

## `// @effect-codegens accessors`

Generates static accessor methods on a service class.

```ts
// @effect-codegens accessors
export class MyService extends Effect.Service<MyService>()("MyService", {
  effect: Effect.gen(function* () {
    return {
      greet: (name: string) => Effect.succeed(`Hello, ${name}`),
    }
  }),
}) {}

// Generated on the class:
// static greet(name: string): Effect.Effect<string, never, MyService>
```

**When to use**: Services whose methods should be callable without importing
`Context.Tag` and calling `.pipe(Effect.flatMap(...))`. The accessor pattern:

```ts
// Without accessors
yield* MyService
  .pipe(Effect.flatMap((svc) => svc.greet("world")))

// With accessors
yield* MyService.greet("world")
```

Applies to: `Effect.Service`, `Context.Tag`, `Effect.Tag`.

**Equivalent IDE action**: `writeTagClassAccessors` refactoring does the same thing
as a one-off. Use the directive when you want accessors to stay in sync automatically
when the service shape changes.

## `// @effect-codegens typeToSchema`

Generates Schema classes from a type alias. One `typeToSchema` directive allowed per file.

```ts
// @effect-codegens typeToSchema
type ToGenerate = {
  UserSchema: User
  TodoSchema: Todo
}

// Generates Schema.TaggedClass (or equivalent) for each entry
```

**When to use**: Migrating existing TypeScript types to Schema at system boundaries.
Useful for batch conversion rather than writing each Schema manually.

## Freshness Checking

The `outdatedEffectCodegen` diagnostic (default: warning, has quick fix) detects when
generated code is stale — the source changed but codegen hasn't been rerun.

Fix options:
- Run `effect-language-service codegen --file <path>` to regenerate
- Use the IDE quick fix
- Run `effect-language-service codegen --project tsconfig.json --force` to regenerate all

## Running Codegen

```bash
# Single file
effect-language-service codegen --file src/services/MyService.ts

# Full project
effect-language-service codegen --project tsconfig.json

# Force regenerate (overwrites all generated blocks)
effect-language-service codegen --project tsconfig.json --force

# Verbose output (shows which files are processed)
effect-language-service codegen --project tsconfig.json --verbose
```

Files are processed in batches of 50. Only files containing `@effect-codegens` are scanned.
