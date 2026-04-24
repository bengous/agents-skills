---
name: bun-cc
description: >
  TypeScript CLI scripts and applications using the Bun runtime. Covers file structure
  conventions, Bun Shell ($), File I/O, testing with bun:test, HTTP servers, SQLite, and
  bundling. NOT for: Node.js/Deno projects, frontend-only React/Vue apps, or npm package
  publishing workflows.
model: opus
effort: medium
allowed-tools:
  - Read
  - Write
  - Edit
  - Grep
  - Glob
  - Bash
  - WebFetch
  - WebSearch
  - mcp__bun__run-bun-script-file
  - mcp__bun__run-bun-eval
  - mcp__bun__run-bun-install
  - mcp__bun__run-bun-script
  - mcp__bun__run-bun-build
  - mcp__bun__run-bun-test
  - mcp__bun__analyze-bun-performance
  - mcp__bun__benchmark-bun-script
  - mcp__bun__start-bun-server
  - mcp__bun__list-servers
  - mcp__bun__stop-server
  - mcp__bun__get-server-logs
  - mcp__bun__get-bun-version
  - mcp__bun__list-bun-versions
  - mcp__bun__select-bun-version
  - mcp__Context7__resolve-library-id
  - mcp__Context7__query-docs
  - mcp__exa__web_search_exa
  - mcp__exa__get_code_context_exa
---

# Bun Development

Use this skill for Bun runtime scripts, CLIs, small services, and Bun-native tooling.
Prefer project conventions over these defaults when the local repo already has a
working Bun shape.

## Navigating References

Load only the reference needed for the task.

| File | Answers |
|---|---|
| `references/bun-api.md` | Shell `$`, spawn, file I/O, archives, utilities |
| `references/testing.md` | `bun:test`, mocks, CLI integration tests, CI flags |
| `references/patterns.md` | CLI logging, confirmation, errors, parallel work |
| `references/http-server.md` | `Bun.serve`, routing, WebSocket upgrades |
| `references/bundler.md` | `Bun.build`, standalone executables, targets |
| `references/sqlite.md` | `bun:sqlite`, prepared statements, transactions |
| `references/crypto.md` | Bun.password hashing, CryptoHasher |

## Task Workflow

1. Inspect local truth first: `package.json`, `bunfig.toml`, `tsconfig.json`, lockfile,
   scripts, current Bun version, and existing test/build commands.
2. Use Bun MCP tools when they are actually loaded. If not, use Bash for local Bun
   commands and fetch official docs for API uncertainty.
3. Keep root code plain TypeScript unless the project already uses a framework. Bun is
   the runtime/toolkit, not an excuse to add abstractions.
4. Export pure functions from CLI entrypoints for direct tests; keep side effects in
   `main()` or thin adapters.
5. Run the smallest project-native validation first (`bun test`, targeted tests,
   typecheck/lint/build as configured), then broaden only when the change touches shared
   behavior.

## Default CLI Shape

Use this shape when the project has no stronger convention:

```
~/.local/bin/
|-- my-script              # Bash wrapper in PATH
`-- my-script-lib/
    |-- my-script.ts       # TypeScript entrypoint
    |-- my-script.test.ts
    |-- package.json
    `-- tsconfig.json
```

```bash
#!/usr/bin/env bash
exec bun "$HOME/.local/bin/my-script-lib/my-script.ts" "$@"
```

This keeps the command name extensionless while leaving TypeScript and tests in a
normal project directory.

## Quick Start Template

```typescript
#!/usr/bin/env bun

const VERSION = "1.0.0";

export function processData(input: string): string {
  return input.trim().toUpperCase();
}

async function main(): Promise<void> {
  const args = Bun.argv.slice(2);

  if (args.includes("--version")) {
    console.log(VERSION);
    process.exit(0);
  }

  if (args.length === 0) {
    console.error("Usage: my-script <input>");
    process.exit(1);
  }

  const result = processData(args[0]);
  console.log(result);
}

if (import.meta.main) {
  main().catch((error: unknown) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
```

```json
{
  "name": "my-script",
  "version": "1.0.0",
  "type": "module",
  "scripts": { "test": "bun test" },
  "devDependencies": { "@types/bun": "latest" }
}
```

```json
{
  "compilerOptions": {
    "lib": ["ESNext"],
    "target": "ESNext",
    "module": "Preserve",
    "moduleDetection": "force",
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "verbatimModuleSyntax": true,
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}
```

## Gotchas

- `Bun.file().exists()` is async. Always `await` it.
- Bun transpiles TypeScript but does not typecheck. Use the project typecheck command.
- Install Bun globals with `@types/bun`; do not use stale `bun-types` tsconfig entries.
- Bun Shell interpolation is escaped by default, but protection ends when handing
  data to another shell such as `bash -c`.
- Use `node:fs/promises` for directory operations not covered by `Bun.file` or
  `Bun.write`.
- Prefer absolute paths or `Bun.which()` when invoking a security-sensitive binary.

## Fetching Documentation

Use official Bun docs when API details matter:

| Need | Source |
|---|---|
| Runtime APIs | `https://bun.sh/docs/runtime/...` |
| Test runner | `https://bun.sh/docs/test` |
| Bundler/executables | `https://bun.sh/docs/bundler` |
| TypeScript config | `https://bun.sh/docs/typescript` |
| Package manager/install | `https://bun.sh/docs/pm` |

!`echo "## Bun MCP"`
!`~/.claude/skills/bun-cc/scripts/probe-bun-mcp.sh`
