# Configuration Reference

All configuration lives in `tsconfig.json` under `compilerOptions.plugins`:

```jsonc
{
  "$schema": "./node_modules/@effect/language-service/schema.json",
  "compilerOptions": {
    "plugins": [
      { "name": "@effect/language-service", /* options here */ }
    ]
  }
}
```

The `$schema` enables JSON autocompletion for all plugin options.

---

## Feature Toggles

```jsonc
{
  "refactors": true,      // Refactoring code actions
  "diagnostics": true,    // Effect diagnostics
  "quickinfo": true,      // Enriched hover info
  "completions": true,    // Effect-specific completions
  "goto": true,           // Enhanced go-to-definition
  "inlays": true,         // Inlay hints for Effect types
  "renames": true          // Rename propagation (class → string identifier)
}
```

## Diagnostic Severity

Per-rule severity in `diagnosticSeverity`. Values: `off`, `error`, `warning`, `message`, `suggestion`.

```jsonc
{
  "diagnosticSeverity": {
    "floatingEffect": "error",
    "asyncFunction": "warning",
    "globalConsole": "off"
  },
  "diagnosticsName": true,         // Show rule name in diagnostic message
  "missingDiagnosticNextLine": "warning"  // Severity for unused suppression comments
}
```

## TSC Patch Behavior

Controls how the patched `tsc` treats Effect diagnostics in its exit code:

```jsonc
{
  "ignoreEffectWarningsInTscExitCode": false,
  "ignoreEffectErrorsInTscExitCode": false,
  "ignoreEffectSuggestionsInTscExitCode": true,   // default: true
  "includeSuggestionsInTsc": true,                 // Show suggestions in tsc output
  "skipDisabledOptimization": false                // Process disabled rules for inline overrides
}
```

## Hover Display

```jsonc
{
  "quickinfoEffectParameters": "whenTruncated",  // always | never | whenTruncated
  "quickinfoMaximumLength": -1                    // -1 = no truncation
}
```

## Import Preferences

```jsonc
{
  "namespaceImportPackages": ["effect", "@effect/*"],  // Prefer import * as X
  "topLevelNamedReexports": "ignore",                  // ignore | follow
  "barrelImportPackages": [],                          // Prefer barrel imports
  "importAliases": { "Array": "Arr" }                 // Custom aliases
}
```

## Layer Graph

```jsonc
{
  "layerGraphFollowDepth": 0,             // Layer dependency resolution depth in hover
  "mermaidProvider": "mermaid.live"        // mermaid.live | mermaid.com | custom url
}
```

## Effect.fn

```jsonc
{
  "effectFn": ["span"]  // untraced | span | suggested-span | inferred-span | no-span
}
```

## Deterministic Keys

```jsonc
{
  "keyPatterns": [
    {
      "target": "service",       // service | error | custom
      "pattern": "default",      // default | default-hashed | package-identifier
      "skipLeadingPath": ["src/"]
    }
  ],
  "extendedKeyDetection": false  // Custom @effect-identifier detection (slower)
}
```

For custom APIs:
```ts
export function Repository(/** @effect-identifier */ identifier: string) {
  return Context.Tag("Repository/" + identifier)
}
```

## Miscellaneous

```jsonc
{
  "noExternal": false,              // Disable external links (mermaid.live etc.)
  "allowedDuplicatedPackages": [],  // Packages allowed to duplicate Effect
  "pipeableMinArgCount": 2          // Min args before pipeable suggestions
}
```

---

## Inline Suppression

### File-level (from comment to end of file)

```ts
// @effect-diagnostics effect/floatingEffect:off
Effect.succeed(1);  // not flagged

// @effect-diagnostics effect/floatingEffect:error
Effect.succeed(1);  // flagged as error
```

### Wildcard (all rules)

```ts
// @effect-diagnostics *:off
Effect.succeed(1);  // nothing flagged

// @effect-diagnostics effect/floatingEffect:error
Effect.succeed(1);  // flagged (specific overrides wildcard)
```

### Next-line only

```ts
// @effect-diagnostics-next-line effect/floatingEffect:off
Effect.succeed(1);  // this line suppressed
Effect.succeed(1);  // flagged normally
```

Unused `@effect-diagnostics-next-line` comments trigger the `missingDiagnosticNextLine`
diagnostic (default: warning).

Accepted severity values: `off`, `error`, `warning`, `message`, `suggestion`.

---

## CI Integration

### Standalone diagnostics (no patch required)

```bash
effect-language-service diagnostics \
  --project tsconfig.json \
  --format github-actions \
  --strict
```

`--strict` makes warnings fail the build. `--format github-actions` produces annotations
that GitHub displays inline on PRs.

### Patched tsc approach

Patch TypeScript, then use `tsc` normally. Control exit behavior:
- `ignoreEffectWarningsInTscExitCode: true` — warnings don't fail `tsc`
- `ignoreEffectSuggestionsInTscExitCode: true` — suggestions don't fail (default)

The standalone `diagnostics` command is simpler and doesn't require patching.

---

## Configuration Profiles

### New project (strict correctness)

Start with defaults. All correctness rules at error, anti-patterns at warning.
No effect-native rules — focus on getting Effect patterns right first.

```jsonc
{
  "name": "@effect/language-service",
  "diagnostics": true,
  "quickinfo": true,
  "completions": true,
  "diagnosticSeverity": {
    "deterministicKeys": "warning"
  }
}
```

### Mature project (comprehensive)

Enable effect-native rules for modules that should be pure Effect.
Tighten anti-patterns to errors. Add style suggestions.

```jsonc
{
  "name": "@effect/language-service",
  "diagnostics": true,
  "quickinfo": true,
  "completions": true,
  "diagnosticSeverity": {
    "anyUnknownInErrorContext": "warning",
    "deterministicKeys": "warning",
    "effectFnImplicitAny": "error",
    "effectInFailure": "error",
    "extendsNativeError": "warning",
    "importFromBarrel": "warning",
    "instanceOfSchema": "warning",
    "missedPipeableOpportunity": "suggestion",
    "missingEffectServiceDependency": "warning",
    "serviceNotAsClass": "warning",
    "strictBooleanExpressions": "warning",
    "strictEffectProvide": "warning"
  }
}
```

### CI-only (minimal gate)

Only correctness errors. Fast, no style noise.

```jsonc
{
  "name": "@effect/language-service",
  "diagnostics": true,
  "refactors": false,
  "quickinfo": false,
  "completions": false,
  "diagnosticSeverity": {}
}
```

Run with: `diagnostics --project tsconfig.json --format github-actions --strict --severity error`

### Example: etch project (mature Effect codebase)

A real-world configuration for a mature Effect project with strict error handling
and progressive effect-native adoption:

```jsonc
{
  "name": "@effect/language-service",
  "diagnostics": true,
  "quickinfo": true,
  "completions": true,
  "ignoreEffectWarningsInTscExitCode": false,
  "diagnosticSeverity": {
    "anyUnknownInErrorContext": "warning",
    "deterministicKeys": "warning",
    "extendsNativeError": "warning",
    "importFromBarrel": "warning",
    "instanceOfSchema": "warning",
    "missedPipeableOpportunity": "suggestion",
    "missingEffectServiceDependency": "warning",
    "effectFnImplicitAny": "error",
    "effectInFailure": "error",
    "effectSucceedWithVoid": "suggestion",
    "nodeBuiltinImport": "off",
    "schemaUnionOfLiterals": "suggestion",
    "serviceNotAsClass": "warning",
    "strictBooleanExpressions": "warning",
    "strictEffectProvide": "warning"
  }
}
```

Notable choices:
- `nodeBuiltinImport: "off"` — project uses `@effect/platform` where appropriate
  but doesn't enforce it everywhere (scripts, test fixtures still use Node directly)
- `effectInFailure: "error"` — promoted from warning because this is always a bug
- `strictEffectProvide: "warning"` — enforces provide-at-entry-points pattern
- `ignoreEffectWarningsInTscExitCode: false` — warnings fail the build

Scripts:
```bash
bun run effect:diagnose    # effect-language-service diagnostics --project tsconfig.json
bun run effect:quickfixes  # effect-language-service quickfixes --project tsconfig.json
```

## Editor Setup

| Editor | Setup |
|---|---|
| **VS Code** | F1 > "TypeScript: Select TypeScript Version" > "Use workspace version" |
| **JetBrains** | Settings > TypeScript > workspace version selector |
| **Neovim (vtsls)** | Follow [plugin activation guide](https://github.com/yioneko/vtsls?tab=readme-ov-file#typescript-plugin-not-activated) |
| **Emacs** | [Setup guide](https://gosha.net/2025/effect-ls-emacs/) |

For **SvelteKit**, the Svelte plugin must be first:
```jsonc
"plugins": [
  { "name": "typescript-svelte-plugin" },
  { "name": "@effect/language-service" }
]
```
