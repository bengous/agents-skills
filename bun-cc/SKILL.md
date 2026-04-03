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

## Overview

This skill covers Bun runtime APIs for CLI scripts and applications. The main content
below focuses on CLI script scaffolding. Load a reference for API details and patterns.

## Navigating References

Start here for CLI scripts. Load a reference when you need API details or patterns.

| File | Answers |
|---|---|
| `references/bun-api.md` | Shell ($), Spawn, File I/O, Archives, Utilities |
| `references/testing.md` | bun:test, matchers, CLI integration testing |
| `references/patterns.md` | Logging, user confirmation, error handling, parallel ops |
| `references/http-server.md` | HTTP server, routing, WebSocket upgrades |
| `references/bundler.md` | Bun.build, compile to executable, cross-compile |
| `references/sqlite.md` | bun:sqlite, prepared statements, transactions |
| `references/crypto.md` | Bun.password hashing, CryptoHasher |

## File Structure Pattern

```
~/.local/bin/
├── my-script              # Bash wrapper (in PATH)
└── my-script-lib/
    ├── my-script.ts       # Main TypeScript implementation
    ├── my-script.test.ts  # Tests
    ├── package.json
    └── tsconfig.json
```

**Bash wrapper** (`~/.local/bin/my-script`):
```bash
#!/usr/bin/env bash
exec bun "$HOME/.local/bin/my-script-lib/my-script.ts" "$@"
```

**Why this structure?**
- Bash wrapper stays in PATH (no .ts extension visible)
- TypeScript files isolated in lib directory
- Tests colocated with implementation
- Clean separation of concerns

## Quick Start Template

### Main Script (`my-script.ts`)

```typescript
#!/usr/bin/env bun

import { $ } from "bun";

// ============================================================================
// CONSTANTS
// ============================================================================

const VERSION = "1.0.0";

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function processData(input: string): string {
  // Pure, testable logic here
  return input.trim().toUpperCase();
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

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

// CLI entry point (only runs when executed directly)
if (import.meta.main) {
  main().catch((e) => {
    console.error(`Error: ${e.message}`);
    process.exit(1);
  });
}
```

### Package Configuration

**package.json:**
```json
{
  "name": "my-script",
  "version": "1.0.0",
  "type": "module",
  "scripts": { "test": "bun test" },
  "devDependencies": { "@types/bun": "latest" }
}
```

**tsconfig.json:**
```json
{
  "compilerOptions": {
    "strict": true,
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "types": ["bun-types"],
    "skipLibCheck": true,
    "noEmit": true
  }
}
```

## Gotchas

1. **`Bun.file().exists()` is async** - Always await it
   ```typescript
   // WRONG
   if (Bun.file("x").exists()) { }
   // CORRECT
   if (await Bun.file("x").exists()) { }
   ```

2. **PATH lookup vs absolute path** - `Bun.spawn(["cmd"])` uses PATH, which may find wrong binary
   ```typescript
   // Use absolute path when it matters
   Bun.spawnSync(["/usr/local/bin/myapp", "--version"]);
   ```

3. **Buffer vs string** - Shell commands return Buffers
   ```typescript
   const { stdout } = await $`cmd`.nothrow().quiet();
   console.log(stdout.toString());  // Convert to string
   ```

4. **Node.js fs for directories** - Bun doesn't have native directory APIs
   ```typescript
   import { readdir, mkdir } from "node:fs/promises";
   await mkdir("path/to/dir", { recursive: true });
   ```

5. **Exit codes** - Use `process.exit(code)` for CLI exit codes
   ```typescript
   process.exit(success ? 0 : 1);
   ```

## Official Documentation

- [Bun Shell ($)](https://bun.sh/docs/runtime/shell)
- [Child Process](https://bun.sh/docs/runtime/child-process)
- [File I/O](https://bun.sh/docs/runtime/file-io)
- [Archive](https://bun.sh/docs/runtime/archive)
- [Testing](https://bun.sh/docs/test/writing)
- [Utilities (which)](https://bun.sh/docs/runtime/utils)
- [HTTP Server](https://bun.sh/docs/api/http)
- [WebSockets](https://bun.sh/docs/api/websockets)
- [SQLite](https://bun.sh/docs/api/sqlite)
- [Bundler](https://bun.sh/docs/bundler)
- [Compile to Executable](https://bun.sh/docs/bundler/executables)
- [Hashing](https://bun.sh/docs/api/hashing)

## Resources

### references/

- `bun-api.md` - Shell ($), Spawn, File I/O, Archives, Utilities
- `testing.md` - bun:test, matchers, CLI integration testing
- `patterns.md` - Logging, confirmation, error handling, parallel ops
- `http-server.md` - Bun.serve, routing, WebSocket
- `bundler.md` - Bun.build, compile, cross-compile
- `sqlite.md` - bun:sqlite, prepared statements
- `crypto.md` - Bun.password, CryptoHasher

### examples/

Production-grade example scripts demonstrating skill patterns at different complexity levels:

| Example | Complexity | Lines | Key Patterns |
|---------|------------|-------|--------------|
| `01-simple/json-keys` | Simple | ~90 | File I/O, CLI args, pure functions, exit codes |
| `02-medium/gh-release` | Medium | ~240 | HTTP fetch, `Bun.Archive`, retry logic, multi-file |
| `03-complex/env-sync` | Complex | ~650 | Subcommands, Web Crypto, prompts, Bun Shell |

Each example includes:
- Full test suite (`bun test`)
- Biome linting (`biome check .`)
- TypeScript strict mode (`tsc`)
- README with usage documentation

!`echo "## Bun MCP"`
!`~/.claude/skills/bun-cc/scripts/probe-bun-mcp.sh`
