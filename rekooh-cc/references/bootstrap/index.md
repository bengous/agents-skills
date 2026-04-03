# Bootstrap — Set Up Hook Infrastructure

Create the directory structure and configuration files needed for hooks in a target repository.

## Automated bootstrap

Run the bootstrap script to set up infrastructure idempotently:

```bash
bun ~/.claude/skills/rekooh-cc/scripts/bootstrap-hooks-infra.ts
```

The script detects the project context and creates the appropriate structure.

## Standalone setup

For standalone hooks (bash, python, or bun), the minimal structure is:

```
<project-root>/
├── .claude/
│   ├── settings.json          # Hook registrations
│   └── hooks/                 # Hook scripts
│       ├── guard-bash.sh      # Example: bash hook
│       ├── guard-paths.py     # Example: python hook
│       └── lint-after-edit.ts # Example: bun hook
```

**What you need:**
1. A `.claude/` directory at the project root
2. A `settings.json` with hook registrations (see [config](../config/index.md))
3. Hook scripts that read JSON from stdin and exit with 0 (allow), 1 (error), or 2 (block)

Shell scripts must be executable (`chmod +x`).

## Opinionated runtime setup

For the typed runtime, the structure includes the runtime dependency:

```
<project-root>/
├── .claude/
│   ├── settings.json
│   └── hooks/
│       ├── src/
│       │   ├── guard-destructive.ts
│       │   └── lint-after-edit.ts
│       ├── run-hook.sh
│       ├── package.json          # claude-hooks linked to rekooh vendored source
│       └── tsconfig.json
```

**What you need:**
1. A `package.json` with `claude-hooks` resolved via npm link to the rekooh vendored source
2. Hook entrypoints that import `defineHook` from `claude-hooks`
3. Settings registrations using `.claude/hooks/run-hook.sh src/<hook>.ts`

Use `scripts/bootstrap-hooks-infra.ts --opinionated` to generate this structure.

## Next steps

- Write your first hook: [authoring](../authoring/index.md)
- Register hooks in settings: [config](../config/index.md)
- See a full bootstrap walkthrough: [complete-project-bootstrap example](../../examples/complete-project-bootstrap/index.md)
