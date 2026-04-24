# CLI Patterns

Reusable patterns for Bun CLI scripts. These are workflow patterns, not API
reference.

## Logging

```typescript
const colors = {
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  reset: "\x1b[0m",
} as const;

export const log = {
  info: (message: string) => console.log(`${colors.blue}info${colors.reset} ${message}`),
  success: (message: string) => console.log(`${colors.green}ok${colors.reset} ${message}`),
  warn: (message: string) => console.error(`${colors.yellow}warn${colors.reset} ${message}`),
  error: (message: string) => console.error(`${colors.red}error${colors.reset} ${message}`),
};
```

Keep library functions silent. Print only in CLI adapters or top-level command
handlers.

## Confirmation

```typescript
export async function confirm(prompt: string, defaultValue = false): Promise<boolean> {
  const suffix = defaultValue ? "[Y/n]" : "[y/N]";
  process.stdout.write(`${prompt} ${suffix} `);

  for await (const line of console) {
    const answer = line.trim().toLowerCase();
    if (answer === "") return defaultValue;
    if (answer === "y" || answer === "yes") return true;
    if (answer === "n" || answer === "no") return false;
    process.stdout.write("Please answer yes or no: ");
  }

  return defaultValue;
}
```

Do not prompt in non-interactive paths unless the command explicitly supports it.
Accept `--yes`, `--no`, or `--force` for automation.

## Error Boundary

```typescript
export async function main(): Promise<void> {
  // command logic
}

if (import.meta.main) {
  main().catch((error: unknown) => {
    log.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
```

Wrap errors with context at the boundary where the operation becomes meaningful:

```typescript
async function readConfig(path: string): Promise<Config> {
  try {
    return await Bun.file(path).json();
  } catch (error) {
    throw new Error(`Failed to read config ${path}`, { cause: error });
  }
}
```

## Parallel Work

Use unbounded `Promise.all` only for small known lists. For filesystem, network, or
subprocess fan-out, add a small concurrency limit.

```typescript
async function mapLimit<T, R>(
  values: readonly T[],
  limit: number,
  fn: (value: T) => Promise<R>,
): Promise<R[]> {
  const results = new Array<R>(values.length);
  let next = 0;

  async function worker(): Promise<void> {
    while (next < values.length) {
      const index = next++;
      results[index] = await fn(values[index] as T);
    }
  }

  await Promise.all(Array.from({ length: Math.min(limit, values.length) }, worker));
  return results;
}
```

## Exit Codes

Prefer returning data from functions and deciding the exit code in `main()`.

```typescript
const ok = await runChecks();
process.exit(ok ? 0 : 1);
```

Use distinct exit codes only when callers are documented to depend on them.
