---
name: effect-language-service-cc
description: >
  Tooling automation for @effect/language-service in Effect projects. Runs CLI commands
  (diagnostics, quickfixes, codegen, overview, layerinfo), interprets diagnostic output,
  applies automated fixes, configures rule severities, and sets up CI integration. Use when
  working with Effect diagnostics, fixing ELS warnings, generating code from directives,
  analyzing layer composition, or configuring tsconfig plugin options. NOT for: architectural
  decisions about Effect adoption (use effect-usage-cc), non-Effect TypeScript, or manual code
  review without ELS tooling.
model: opus
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
  - mcp__Context7__resolve-library-id
  - mcp__Context7__query-docs
  - mcp__exa__web_search_exa
---

# Effect Language Service

Operational guide for `@effect/language-service` CLI and configuration. This skill handles
**tooling**: running commands, interpreting output, applying fixes, configuring projects.
For architecture decisions about when/how to use Effect, see `effect-usage-cc` instead.

## Command Decision Table

Always run from the **project root** via the local binary (`bunx effect-language-service` or
`./node_modules/.bin/effect-language-service`). Commands like `check` resolve paths relative
to cwd — running from a subdirectory breaks them.

| Intent | Command | Key flags |
|---|---|---|
| Check for Effect issues | `diagnostics --project tsconfig.json` | `--format pretty` human, `--format json` parseable |
| Check single file | `diagnostics --file <path>` | Faster than full project scan |
| See available fixes | `quickfixes --project tsconfig.json` | `--code <rule>` filter by rule |
| Fix specific issue | `quickfixes --file <path> --line <n> --fix <fixName>` | Shows diff to apply |
| Generate from directives | `codegen --project tsconfig.json` | `--force` regenerates all |
| Understand project structure | `overview --project tsconfig.json` | `--max-symbol-depth 2` (depth 3 adds noise) |
| Analyze layer composition | `layerinfo --file <path> --name <Layer>` | Outputs composition order |
| Check TS patch status | `check` | Verifies patch version |
| Patch TypeScript for tsc | `patch` | `--force` re-patches |
| Interactive setup | `setup` | Wizard for first-time config |
| Configure rule severities | `config` | Interactive severity editor |
| CI gate | `diagnostics --format github-actions --strict` | Non-zero exit on warnings |

## Interpreting Output

### Diagnostic format

```
src/service.ts:42:3 effect(floatingEffect): This Effect is not yielded or assigned
```

Structure: `file:line:col effect(ruleName): message`. The `ruleName` maps directly to the
`diagnosticSeverity` config key in tsconfig.json and to the inline suppression syntax.

### Quickfix format

Output: diagnostic line followed by one or more unified diffs per available fix. Each fix
has a `fixName` (e.g., `floatingEffect_yieldStar`) that can be used with `--fix` to filter.

### Severity levels

- **error** — Must fix. Blocks CI (always non-zero exit)
- **warning** — Should fix. Blocks CI only with `--strict`
- **message/suggestion** — Informational. Never blocks CI even with `--strict` (emits
  `::notice` in github-actions format, not `::error`). Only blocks tsc if `includeSuggestionsInTsc` enabled

## Key Workflows

### Fix all issues in a file

```bash
# 1. See what's wrong
effect-language-service diagnostics --file src/service.ts --format pretty

# 2. See available fixes with diffs
effect-language-service quickfixes --file src/service.ts

# 3. Filter to a specific rule
effect-language-service quickfixes --file src/service.ts --code floatingEffect
```

Review the diffs before applying. Correctness fixes (floatingEffect, missingReturnYieldStar)
are safe to apply. Style fixes (effectMapVoid, unnecessaryPipe) are preferences — check
project conventions.

### Onboard a new project

```bash
# 1. Interactive setup (patches TS, configures tsconfig)
effect-language-service setup

# 2. Persist patch across installs
# Add to package.json scripts: "prepare": "effect-language-service patch"

# 3. Baseline scan
effect-language-service diagnostics --project tsconfig.json --format pretty
```

### LLM context generation

`overview` and `layerinfo` produce text output (no `--format json` — only `diagnostics`
supports `--format`). Both emit ANSI spinner noise that must be stripped.

**Clean extraction pattern** (use for all text-output commands):

```bash
# Strip ANSI codes and spinner lines from any ELS text command
els_clean() {
  effect-language-service "$@" 2>/dev/null \
    | sed 's/\x1b\[[0-9;]*m//g' \
    | grep -v "^Processing file"
}

# Project map: services, layers, yieldable errors
els_clean overview --project tsconfig.json --max-symbol-depth 2

# Specific layer: provides, requires, composition order
els_clean layerinfo --file src/layers.ts --name AppLayer

# Full snapshot: overview + diagnostics + quickfixes in one shot
els_clean overview --project tsconfig.json --max-symbol-depth 2 > /tmp/els-overview.txt
effect-language-service diagnostics --project tsconfig.json --format json > /tmp/els-diag.json
els_clean quickfixes --project tsconfig.json > /tmp/els-fixes.txt
```

`overview` groups exports into "Yieldable Errors", "Services", "Layers" with file+line,
type info, and JSDoc. **Warning**: each symbol appears at every re-export point (barrel
files, index.ts). Deduplicate by source `file:line` — expect ~40% noise from duplicates
in large projects.

`layerinfo` outputs a suggested `Layer.provide` / `Layer.provideMerge` composition.
Requires a **named, exported Layer** — anonymous or inline layers are not supported.
Tip: write all layers in `Layer.mergeAll(...)`, run `layerinfo`, use the suggested order.

## Guardrails

1. **Scope your scans** — Use `--file` when working on a single file. Full `--project` scans
   are slow in large codebases and produce noise from files you're not touching.

2. **Review quickfix diffs** — Correctness fixes are generally safe. Style fixes may conflict
   with project conventions. Anti-pattern fixes sometimes need manual adjustment.

3. **Monorepo tsconfig** — Always specify `--project` explicitly. Without it, ELS infers
   the tsconfig which may be wrong in monorepos with multiple configs.

4. **codegen --force caution** — Regenerates ALL directives in the project. Use `--file` to
   scope to a single file. Manual edits to generated blocks will be overwritten.

5. **Patch persistence** — The TS patch modifies `node_modules/typescript/`. It disappears
   after `npm install`/`bun install`. Use `"prepare": "effect-language-service patch"` to
   persist it.

6. **diagnostics vs tsc** — Without patching, `tsc` does NOT run Effect diagnostics. The
   `diagnostics` CLI command works without patching. The patch is only needed for `tsc`
   integration and IDE features.

7. **ANSI spinner pollution** — `overview`, `layerinfo`, and `quickfixes` emit ANSI escape
   codes and "Processing file" spinner lines mixed into stdout. Always pipe through the
   `els_clean` pattern (strip ANSI + filter spinner) when capturing output programmatically.

8. **--format is diagnostics-only** — Only `diagnostics` supports `--format json|text|pretty|github-actions`.
   All other commands (`overview`, `layerinfo`, `quickfixes`, `codegen`) output unstructured
   text. Do not waste time trying `--format` on them.

9. **codegen with no directives** — If no files contain `@effect-codegens`, `codegen` throws
   `NoFilesToCodegenError` instead of exiting cleanly. This is not a real error — the project
   simply has no codegen directives. Check before running.

10. **config/setup file picker** — Both `config` and `setup` are interactive and list ALL `.json`
    files in the project, not just tsconfig files. This is a known UX issue — select the
    correct tsconfig manually.

## Fetching Documentation

This skill covers tooling operations. For current API details or version-specific changes:

| Need | Tool |
|---|---|
| ELS release notes, new rules | `mcp__exa__web_search_exa` — search Effect-TS/language-service |
| Effect API docs | `mcp__Context7__query-docs` — resolve `effect` library |
| Pattern guidance | Load `effect-usage-cc` skill |

## Navigating References

Load references when the command table and workflows above are insufficient.

| File | Answers |
|---|---|
| `references/cli-commands.md` | Full syntax for each command? All flags and defaults? |
| `references/diagnostic-rules.md` | What does rule X mean? How to fix it? Default severity? |
| `references/refactorings.md` | Which IDE refactoring to use? async→Effect variants? |
| `references/codegen-directives.md` | How to use @effect-codegens? When to annotate/accessors/typeToSchema? |
| `references/configuration.md` | All tsconfig options? Severity profiles? CI setup? Inline suppression? |

!`echo "## ELS Project Context"`
!`~/.claude/skills/effect-language-service-cc/scripts/probe-els.sh`
