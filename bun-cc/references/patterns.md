# CLI Patterns

Reusable patterns for Bun CLI scripts. These are not API references — they are
idiomatic solutions to common CLI needs.

## Logging with Colors

```typescript
const colors = {
  red: "\x1b[0;31m",
  green: "\x1b[0;32m",
  yellow: "\x1b[1;33m",
  blue: "\x1b[0;34m",
  reset: "\x1b[0m",
} as const;

const log = {
  info: (msg: string) => console.log(`${colors.blue}i${colors.reset} ${msg}`),
  success: (msg: string) => console.log(`${colors.green}v${colors.reset} ${msg}`),
  warning: (msg: string) => console.log(`${colors.yellow}!${colors.reset} ${msg}`),
  error: (msg: string) => console.error(`${colors.red}x${colors.reset} ${msg}`),
};
```

## User Confirmation

```typescript
async function confirm(prompt: string): Promise<boolean> {
  process.stdout.write(`${prompt} [Y/n] `);
  for await (const line of console) {
    const answer = line.trim().toLowerCase();
    return answer !== "n";
  }
  return false;
}

if (await confirm("Proceed?")) {
  // do action
}
```

## Error Handling

```typescript
async function main(): Promise<void> {
  try {
    // main logic
  } catch (e) {
    log.error(e instanceof Error ? e.message : String(e));
    process.exit(1);
  }
}

if (import.meta.main) {
  main();
}
```

## Parallel Operations

```typescript
// Process multiple files in parallel
const files = ["a.json", "b.json", "c.json"];
const results = await Promise.all(files.map(processFile));
process.exit(results.every(Boolean) ? 0 : 1);
```
