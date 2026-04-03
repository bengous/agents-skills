# Typed Hook Project

Claude Code hooks using the `claude-hooks` typed runtime.

## Setup

1. Copy this directory into your project (e.g., `.claude/hooks/`):
   ```sh
   cp -r <scaffold-path>/opinionated/ .claude/hooks/
   ```

2. Update the `claude-hooks` dependency path in `package.json` to point to your local `_hooks-lib`:
   ```json
   "claude-hooks": "link:../../path-to/_hooks-lib"
   ```

3. Install dependencies:
   ```sh
   cd .claude/hooks && bun install
   ```

4. Write your hooks in `src/` using `defineHook()`:
   ```ts
   import { defineHook } from "claude-hooks";

   defineHook("PreToolUse", {
     matcher: "Bash",
     check: (input, respond) => {
       // your logic here
       return respond.allow();
     },
   });
   ```

5. Register in `.claude/settings.json`:
   ```json
   {
     "hooks": {
       "PreToolUse": [
         { "hooks": [".claude/hooks/run-hook.sh src/my-guard.ts"] }
       ]
     }
   }
   ```

## Scripts

- `bun run typecheck` — Type-check all hooks
- `bun run lint` — Lint with Biome
- `bun run test` — Run tests
- `bun run validate` — All checks
