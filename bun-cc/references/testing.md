# Testing with bun:test

Bun's test runner is TypeScript-first and Jest-compatible enough for most CLI and
library tests. Prefer direct unit tests for pure functions and subprocess tests for
real CLI behavior.

## Core API

```typescript
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  mock,
  spyOn,
  test,
} from "bun:test";

describe("processData", () => {
  test("trims and uppercases input", () => {
    expect(processData("  hello  ")).toBe("HELLO");
  });
});
```

Common matchers: `toBe`, `toEqual`, `toStrictEqual`, `toContain`, `toMatch`,
`toMatchObject`, `toHaveProperty`, `toThrow`, `resolves`, `rejects`,
`toMatchSnapshot`, `toMatchInlineSnapshot`, plus mock call matchers such as
`toHaveBeenCalled` and `toHaveBeenCalledTimes`.

## Test Selection and Execution

```typescript
test.skip("documents skipped behavior", () => {});
test.only("focuses one test while debugging", () => {});
test.todo("covers config migration");
test.if(process.platform !== "win32")("uses POSIX permissions", () => {});

test.concurrent("parallel-safe case", async () => {});
test.serial("shared state case", async () => {});
```

Useful CLI flags:

```bash
bun test
bun test path/or/name
bun test --test-name-pattern "config"
bun test --concurrent --max-concurrency 4
bun test --rerun-each 50
bun test --retry 2
bun test --randomize --seed 12345
bun test --reporter=junit --reporter-outfile=./bun.xml
bun test --coverage
```

Use `--rerun-each` or `--randomize --seed` when chasing order-dependent or flaky
tests. Use `test.serial` for shared state that cannot be isolated cheaply.

## Parameterized and Async Tests

```typescript
test.each([
  [" abc ", "ABC"],
  ["x", "X"],
])("processData(%j) returns %j", (input, expected) => {
  expect(processData(input)).toBe(expected);
});

test("rejects bad input", async () => {
  await expect(loadConfig("missing.json")).rejects.toThrow("missing.json");
});
```

## Mocks, Spies, and Modules

```typescript
const fetchUser = mock(async (id: string) => ({ id }));

test("calls dependency", async () => {
  await fetchUser("42");
  expect(fetchUser).toHaveBeenCalledWith("42");
});
```

```typescript
const spy = spyOn(console, "error").mockImplementation(() => {});
try {
  logFailure(new Error("boom"));
  expect(spy).toHaveBeenCalled();
} finally {
  spy.mockRestore();
}
```

Module mocks are hoisted by Bun and can override already-imported modules:

```typescript
mock.module("./github", () => ({
  fetchRelease: mock(async () => ({ tag_name: "v1.0.0" })),
}));
```

Prefer dependency injection for new code. Use `mock.module()` when replacing an
imported external boundary is simpler than reshaping production code.

## CLI Integration Tests

Run the real entrypoint in a temp directory and assert exit code plus output.

```typescript
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

describe("my-script CLI", () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), "my-script-"));
  });

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true });
  });

  test("prints processed file content", async () => {
    const input = join(tempDir, "input.txt");
    await writeFile(input, "hello");

    const proc = Bun.spawn(["bun", "./my-script.ts", input], {
      cwd: import.meta.dir,
      stdout: "pipe",
      stderr: "pipe",
    });

    const [stdout, stderr, exitCode] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
      proc.exited,
    ]);

    expect(stderr).toBe("");
    expect(exitCode).toBe(0);
    expect(stdout).toContain("HELLO");
  });
});
```

Guidelines:

- Test pure exported functions directly.
- Use subprocess tests for argument parsing, exit codes, stdio, and filesystem effects.
- Use temp directories for file tests and `force: true` in cleanup.
- Capture stderr in negative-path tests; do not rely only on exit code.
- Avoid snapshotting noisy CLI output unless the command is intentionally stable.

## Official Docs

- `https://bun.sh/docs/test`
- `https://bun.sh/docs/test/mocks`
