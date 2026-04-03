# CLI Commands Reference

Binary: `effect-language-service`. Always run from the **project root** via the local
binary (`bunx effect-language-service`, `./node_modules/.bin/effect-language-service`,
or via package.json scripts). Commands resolve paths relative to cwd — running from a
subdirectory breaks `check` and causes wrong tsconfig inference.

---

## `setup`

Interactive wizard. Selects a tsconfig.json, evaluates current state, asks which features
to enable, calculates diffs, applies after confirmation.

```bash
effect-language-service setup
```

Best for first-time configuration. Handles plugin entry, patch, and rule selection in one flow.

**Known UX issue**: The file picker lists ALL `.json` files in the project, not just
tsconfig files. Select the correct tsconfig manually.

## `config`

Interactive severity configurator. Lets you toggle individual diagnostic rules on/off
and set their severity.

```bash
effect-language-service config
```

**Known UX issue**: Same as `setup` — lists ALL `.json` files, not just tsconfig files.

## `patch` / `unpatch`

Patches the local TypeScript installation so `tsc` executes Effect diagnostics.
Without patching, `tsc` ignores all Effect rules — only the `diagnostics` CLI and
IDE plugin work without patching.

```bash
effect-language-service patch [--dir <path>] [--module tsc|typescript] [--force]
effect-language-service unpatch [--dir <path>] [--module tsc|typescript]
```

| Flag | Default | Description |
|---|---|---|
| `--dir <path>` | `./node_modules/typescript` | TypeScript package directory |
| `--module tsc\|typescript` | Both | Which module(s) to patch (repeatable) |
| `--force` | false | Re-patch even if already patched |

After patching: delete `.tsbuildinfo` files and rebuild.

Persist across installs:
```jsonc
{ "scripts": { "prepare": "effect-language-service patch" } }
```

## `check`

Verifies whether TypeScript is patched and at which version.

```bash
effect-language-service check [--dir <path>]
```

**cwd-sensitive**: `--dir` defaults to `./node_modules/typescript` relative to cwd.
Running from a subdirectory will fail silently or check the wrong TypeScript installation.

The command is idempotent — safe to run repeatedly. `patch` without `--force` skips if
already patched ("already patched, skipped").

Output example:
```
/node_modules/typescript/lib/typescript.js patched with version 0.84.2
/node_modules/typescript/lib/_tsc.js not patched.
```

## `diagnostics`

Runs Effect diagnostics in CLI. Works without patching TypeScript.

```bash
effect-language-service diagnostics \
  [--file <path>] \
  [--project <tsconfig>] \
  [--format pretty|json|text|github-actions] \
  [--strict] \
  [--severity error,warning,message] \
  [--progress] \
  [--lspconfig '<json>']
```

| Flag | Default | Description |
|---|---|---|
| `--file <path>` | — | Single file mode |
| `--project <tsconfig>` | Inferred | Explicit tsconfig (always specify in monorepos) |
| `--format` | `pretty` | `pretty` (color+context), `text`, `json`, `github-actions` |
| `--strict` | false | Warnings treated as errors for exit code |
| `--severity` | All | Filter: comma-separated `error,warning,message` |
| `--progress` | false | Show progress on stderr |
| `--lspconfig '<json>'` | — | Inline config override |

**Exit code**: non-zero if errors detected, or warnings with `--strict`. `message` severity
**never** causes non-zero exit even with `--strict` — messages emit `::notice` annotations
in github-actions format, not `::error`. To block on messages too, promote them to warning
via `diagnosticSeverity` in tsconfig.

**Performance**: `--file` is ~10x faster than `--project` (3s vs 30s on a 350-file project).
Prefer `--file` when working on specific files.

**CI usage**: `diagnostics --project tsconfig.json --format github-actions --strict`

## `quickfixes`

Shows diagnostics with available quick fixes, including unified diffs.

```bash
effect-language-service quickfixes \
  [--file <path>] \
  [--project <tsconfig>] \
  [--code <name|code>] \
  [--line <n>] \
  [--column <n>] \
  [--fix <fixName>]
```

| Flag | Description |
|---|---|
| `--code <name>` | Filter by rule name (e.g., `floatingEffect`) or numeric code |
| `--line <n>` | Filter by line (1-based) |
| `--column <n>` | Filter by column (requires `--line`) |
| `--fix <fixName>` | Filter by fix name (e.g., `floatingEffect_yieldStar`) |

Output: `file:line:col effect(ruleName): message` then unified diffs per fix.
No `--format` flag — output is unstructured text. ANSI cleaning required (same as overview).

## `codegen`

Processes `@effect-codegens` directives and writes generated code to files.

```bash
effect-language-service codegen \
  [--file <path>] \
  [--project <tsconfig>] \
  [--verbose] \
  [--force]
```

Processes files in batches of 50. Only scans files containing `@effect-codegens`.
Use `--force` to regenerate even when output appears up-to-date.
Use `--file` to scope to a single file (avoids touching unrelated generated code).

**If no files contain `@effect-codegens`**, the command throws `NoFilesToCodegenError`
instead of exiting cleanly with 0. This is not a real error — the project simply has no
codegen directives. Check for directive usage before running:
`grep -r "@effect-codegens" src/`

## `overview`

Summarizes a project's Effect exports: services, layers, yieldable errors.
Output is unstructured text (no `--format json`). Ideal for LLM context after cleaning.

```bash
effect-language-service overview \
  [--file <path>] \
  [--project <tsconfig>] \
  [--max-symbol-depth <n>]
```

| Flag | Default | Description |
|---|---|---|
| `--max-symbol-depth <n>` | 3 | Traversal depth (0 = root exports only) |

Output groups: "Yieldable Errors", "Services", "Layers". Each entry: name, file+line,
type, JSDoc.

### Practical notes

- **Use `--max-symbol-depth 1` or `2`** for global overviews. Depth 3 (default) adds
  deep type expansion noise without proportional value.
- **Use `--file` for targeted analysis** — `overview --file src/services/` is much faster
  and more useful than scanning the entire project.
- **Deduplication required**: ELS reports each symbol at every export point. Barrel files
  and `index.ts` re-exports cause ~40% duplicates in large projects. Deduplicate by
  source `file:line` in the output.
- **ANSI cleaning required**: Output contains spinner escape codes and "Processing file"
  lines on stdout. Strip with: `2>/dev/null | sed 's/\x1b\[[0-9;]*m//g' | grep -v "^Processing file"`

## `layerinfo`

Analyzes an exported layer: what it provides, what it requires, and suggests a
composition order.

```bash
effect-language-service layerinfo \
  --file <path> \
  --name <LayerName> \
  [--outputs 1,2,3]
```

| Flag | Required | Description |
|---|---|---|
| `--file` | Yes | File containing the layer |
| `--name` | Yes | Name of the exported layer |
| `--outputs` | No | 1-based indices (csv) to filter output types — changes composition style |

Output: layer type, "Provides (N):", "Requires (N):", indexed output types, and a
suggested `export const X = A.pipe(Layer.provide(B), ...)` composition block.

### Practical notes

- **Named exports only** — `layerinfo` requires a named, exported Layer. Anonymous or
  inline layers (`Layer.effect(Tag, ...)` not assigned to an export) are not analyzable.
- **Composition may truncate** — The suggested composition order can be cut off mid-sentence
  for complex layer graphs (known formatting issue in ELS 0.84.x). Verify the full
  composition manually when this happens.
- **`--outputs` changes composition style** — Without `--outputs`, the composition uses
  `Layer.provideMerge` (services pass through to the output type). With `--outputs` filtering
  a subset, unselected services don't need exposure so ELS switches to `Layer.provide`
  (services are consumed, not exposed). This is intentional: `provideMerge` passes through,
  `provide` consumes.
- **Single-layer shortcut** — If the layer has no dependencies to compose (Requires = 0 or
  only one layer in the graph), no composition is suggested — there's nothing to compose.
- **ANSI cleaning required** — Same spinner pollution as `overview`. Strip before parsing.
- **No `--format`** — Output is unstructured text only.

**Workflow tip**: Write all layers in `Layer.mergeAll(...)`, run `layerinfo` on the
result, then use the suggested composition order.
