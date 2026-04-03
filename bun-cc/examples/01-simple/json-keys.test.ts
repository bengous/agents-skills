import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { extractKeys, parseArgs } from "./json-keys";

// ============================================================================
// UNIT TESTS
// ============================================================================

describe("extractKeys", () => {
  test("extracts flat keys", () => {
    expect(extractKeys({ a: 1, b: 2 })).toEqual(["a", "b"]);
  });

  test("extracts nested keys with dot notation", () => {
    expect(extractKeys({ user: { name: "x", age: 30 } })).toEqual([
      "user",
      "user.name",
      "user.age",
    ]);
  });

  test("limits depth", () => {
    const obj = { a: { b: { c: 1 } } };
    expect(extractKeys(obj, 1)).toEqual(["a"]);
    expect(extractKeys(obj, 2)).toEqual(["a", "a.b"]);
  });

  test("filters by regex", () => {
    const obj = { userName: 1, userAge: 2, id: 3 };
    expect(extractKeys(obj, -1, /^user/)).toEqual(["userName", "userAge"]);
  });

  test("handles empty object", () => {
    expect(extractKeys({})).toEqual([]);
  });

  test("skips arrays", () => {
    expect(extractKeys({ items: [1, 2, 3] })).toEqual(["items"]);
  });

  test("handles null and primitives", () => {
    expect(extractKeys(null)).toEqual([]);
    expect(extractKeys("string")).toEqual([]);
    expect(extractKeys(123)).toEqual([]);
  });
});

describe("parseArgs", () => {
  test("parses file argument", () => {
    expect(parseArgs(["file.json"])).toMatchObject({ file: "file.json" });
  });

  test("parses --count flag", () => {
    expect(parseArgs(["file.json", "--count"])).toMatchObject({ count: true });
  });

  test("parses --depth with value", () => {
    expect(parseArgs(["--depth", "2", "file.json"])).toMatchObject({
      depth: 2,
    });
  });

  test("parses --filter with pattern", () => {
    expect(parseArgs(["file.json", "--filter", "^user"])).toMatchObject({
      filter: "^user",
    });
  });

  test("parses all flags together", () => {
    const result = parseArgs([
      "file.json",
      "--count",
      "--depth",
      "3",
      "--filter",
      "test",
    ]);
    expect(result).toEqual({
      file: "file.json",
      count: true,
      depth: 3,
      filter: "test",
    });
  });

  test("returns defaults when no args", () => {
    expect(parseArgs([])).toEqual({
      file: undefined,
      count: false,
      depth: -1,
      filter: undefined,
    });
  });
});

// ============================================================================
// CLI INTEGRATION TESTS
// ============================================================================

describe("CLI", () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), "json-keys-test-"));
  });

  afterEach(async () => {
    await rm(tempDir, { recursive: true });
  });

  const run = async (...args: string[]) => {
    const proc = Bun.spawn(["bun", "./json-keys.ts", ...args], {
      cwd: import.meta.dir,
      stdout: "pipe",
      stderr: "pipe",
    });
    return {
      exitCode: await proc.exited,
      stdout: await new Response(proc.stdout).text(),
      stderr: await new Response(proc.stderr).text(),
    };
  };

  test("shows usage when no file", async () => {
    const { exitCode, stderr } = await run();
    expect(exitCode).toBe(1);
    expect(stderr).toContain("Usage:");
  });

  test("errors on missing file", async () => {
    const { exitCode, stderr } = await run("nonexistent.json");
    expect(exitCode).toBe(1);
    expect(stderr).toContain("File not found");
  });

  test("errors on invalid JSON", async () => {
    const testFile = join(tempDir, "invalid.json");
    await Bun.write(testFile, "not json");
    const { exitCode, stderr } = await run(testFile);
    expect(exitCode).toBe(1);
    expect(stderr).toContain("Invalid JSON");
  });

  test("errors on invalid regex", async () => {
    const testFile = join(tempDir, "test.json");
    await Bun.write(testFile, "{}");
    const { exitCode, stderr } = await run(testFile, "--filter", "[invalid");
    expect(exitCode).toBe(1);
    expect(stderr).toContain("Invalid regex");
  });

  test("lists keys from valid JSON", async () => {
    const testFile = join(tempDir, "test.json");
    await Bun.write(testFile, '{"a":1,"b":2}');
    const { exitCode, stdout } = await run(testFile);
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("a\nb");
  });

  test("counts keys with --count", async () => {
    const testFile = join(tempDir, "test.json");
    await Bun.write(testFile, '{"a":1,"b":2}');
    const { exitCode, stdout } = await run(testFile, "--count");
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("2");
  });

  test("limits depth with --depth", async () => {
    const testFile = join(tempDir, "test.json");
    await Bun.write(testFile, '{"a":{"b":{"c":1}}}');
    const { exitCode, stdout } = await run(testFile, "--depth", "2");
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("a\na.b");
  });

  test("filters with --filter", async () => {
    const testFile = join(tempDir, "test.json");
    await Bun.write(testFile, '{"userName":1,"userId":2,"other":3}');
    const { exitCode, stdout } = await run(testFile, "--filter", "^user");
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("userName\nuserId");
  });
});
