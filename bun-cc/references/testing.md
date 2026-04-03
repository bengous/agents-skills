# Testing with bun:test

Bun's built-in test runner. Covers API reference, patterns for testing CLI scripts,
and test organization guidelines.

## Basic Syntax

```typescript
import { describe, test, expect, beforeEach, afterEach } from "bun:test";

test("basic assertion", () => {
  expect(2 + 2).toBe(4);
});

describe("group", () => {
  test("nested test", () => {
    expect(true).toBeTruthy();
  });
});
```

## Common Matchers

| Category | Matchers |
|----------|----------|
| Equality | `.toBe()`, `.toEqual()`, `.toStrictEqual()` |
| Truthiness | `.toBeNull()`, `.toBeUndefined()`, `.toBeDefined()`, `.toBeTruthy()`, `.toBeFalsy()` |
| Numbers | `.toBeGreaterThan()`, `.toBeLessThan()`, `.toBeCloseTo()` |
| Strings | `.toMatch(regex)`, `.toContain(substring)` |
| Arrays | `.toContain()`, `.toHaveLength()` |
| Objects | `.toHaveProperty()`, `.toMatchObject()` |
| Exceptions | `.toThrow()`, `.toThrow(message)` |
| Snapshots | `.toMatchSnapshot()`, `.toMatchInlineSnapshot()` |

## Async Tests

```typescript
test("async/await", async () => {
  const result = await Promise.resolve(42);
  expect(result).toBe(42);
});

test("promise matchers", async () => {
  await expect(Promise.resolve(42)).resolves.toBe(42);
  await expect(Promise.reject(new Error("fail"))).rejects.toThrow("fail");
});
```

## Parametrized Tests

```typescript
test.each([
  [1, 2, 3],
  [2, 3, 5],
])("%d + %d = %d", (a, b, expected) => {
  expect(a + b).toBe(expected);
});

test.each([
  { a: 1, b: 2, sum: 3 },
  { a: 2, b: 3, sum: 5 },
])("$a + $b = $sum", ({ a, b, sum }) => {
  expect(a + b).toBe(sum);
});
```

## Lifecycle Hooks

```typescript
beforeEach(async () => {
  // setup per test
});

afterEach(async () => {
  // teardown per test
});

beforeAll(() => { /* setup once */ });
afterAll(() => { /* teardown once */ });
```

## Test Modifiers

```typescript
test.skip("skipped", () => {});
test.only("isolated", () => {});
test.todo("not implemented");
test.if(condition)("conditional", () => {});
```

## Testing Pure Functions

Import exported pure functions directly from the main script:

```typescript
import { describe, test, expect } from "bun:test";
import { processData } from "./my-script";

describe("processData", () => {
  test("trims and uppercases input", () => {
    expect(processData("  hello  ")).toBe("HELLO");
  });

  test.each([
    ["abc", "ABC"],
    ["123", "123"],
    ["  x  ", "X"],
  ])("processData(%j) -> %j", (input, expected) => {
    expect(processData(input)).toBe(expected);
  });
});
```

## CLI Integration Testing

Test CLI scripts as subprocesses with temp directories for isolation:

```typescript
import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

describe("CLI integration", () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), "test-"));
  });

  afterEach(async () => {
    await rm(tempDir, { recursive: true });
  });

  test("processes file correctly", async () => {
    const inputFile = join(tempDir, "input.txt");
    await writeFile(inputFile, "test data");

    const proc = Bun.spawn(["bun", "./my-script.ts", inputFile], {
      cwd: import.meta.dir,
    });
    const exitCode = await proc.exited;

    expect(exitCode).toBe(0);
  });

  test("exits with error when no args", async () => {
    const proc = Bun.spawn(["bun", "./my-script.ts"], {
      cwd: import.meta.dir,
      stderr: "pipe",
    });
    const exitCode = await proc.exited;
    const stderr = await new Response(proc.stderr).text();

    expect(exitCode).toBe(1);
    expect(stderr).toContain("Usage:");
  });
});
```

## Test Organization

1. Unit tests for pure exported functions
2. Integration tests for CLI behavior
3. Use temp directories for file tests
4. Always clean up in afterEach

## Official Docs

- [Testing](https://bun.sh/docs/test/writing)
