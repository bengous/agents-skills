import { describe, expect, test } from "bun:test";
import { computeDiff, parseEnvFile } from "../src/commands/diff";
import { getProjectName } from "../src/commands/init";
import { parseArgs } from "../src/index";
import { hashPath } from "../src/lib/config";

describe("hashPath", () => {
  test("returns consistent hash for same path", () => {
    const hash1 = hashPath("/home/user/project");
    const hash2 = hashPath("/home/user/project");
    expect(hash1).toBe(hash2);
  });

  test("returns different hash for different paths", () => {
    const hash1 = hashPath("/home/user/project1");
    const hash2 = hashPath("/home/user/project2");
    expect(hash1).not.toBe(hash2);
  });

  test("returns 16 character hash", () => {
    const hash = hashPath("/some/path");
    expect(hash.length).toBe(16);
  });
});

describe("getProjectName", () => {
  test("extracts basename from path", () => {
    expect(getProjectName("/home/user/my-project")).toBe("my-project");
  });
});

describe("parseEnvFile", () => {
  test("parses key=value pairs", () => {
    const content = "KEY1=value1\nKEY2=value2";
    const vars = parseEnvFile(content);
    expect(vars.get("KEY1")).toBe("value1");
    expect(vars.get("KEY2")).toBe("value2");
  });

  test("ignores comments and empty lines", () => {
    const content = "# Comment\nKEY=value\n\n  # Another comment";
    const vars = parseEnvFile(content);
    expect(vars.size).toBe(1);
    expect(vars.get("KEY")).toBe("value");
  });

  test("handles values with equals signs", () => {
    const content = "URL=https://example.com?foo=bar";
    const vars = parseEnvFile(content);
    expect(vars.get("URL")).toBe("https://example.com?foo=bar");
  });
});

describe("computeDiff", () => {
  test("detects added keys", () => {
    const local = new Map([
      ["A", "1"],
      ["B", "2"],
    ]);
    const stored = new Map([["A", "1"]]);
    const { added, removed, changed } = computeDiff(local, stored);
    expect(added).toEqual(["B"]);
    expect(removed).toEqual([]);
    expect(changed).toEqual([]);
  });

  test("detects removed keys", () => {
    const local = new Map([["A", "1"]]);
    const stored = new Map([
      ["A", "1"],
      ["B", "2"],
    ]);
    const { added, removed, changed } = computeDiff(local, stored);
    expect(added).toEqual([]);
    expect(removed).toEqual(["B"]);
    expect(changed).toEqual([]);
  });

  test("detects changed values", () => {
    const local = new Map([["A", "new"]]);
    const stored = new Map([["A", "old"]]);
    const { added, removed, changed } = computeDiff(local, stored);
    expect(added).toEqual([]);
    expect(removed).toEqual([]);
    expect(changed).toEqual(["A"]);
  });
});

describe("parseArgs", () => {
  test("parses command", () => {
    const result = parseArgs(["init"]);
    expect(result.command).toBe("init");
  });

  test("parses help flag", () => {
    const result = parseArgs(["--help"]);
    expect(result.help).toBe(true);
  });

  test("parses version flag", () => {
    const result = parseArgs(["--version"]);
    expect(result.version).toBe(true);
  });

  test("parses command with subArgs", () => {
    const result = parseArgs(["config", "set", "remote", "http://foo"]);
    expect(result.command).toBe("config");
    expect(result.subArgs).toEqual(["set", "remote", "http://foo"]);
  });
});

describe("parseArgs edge cases", () => {
  test("handles empty args", () => {
    const result = parseArgs([]);
    expect(result.command).toBeUndefined();
    expect(result.help).toBe(false);
    expect(result.version).toBe(false);
  });

  test("handles short help flag", () => {
    const result = parseArgs(["-h"]);
    expect(result.help).toBe(true);
  });

  test("ignores flags before command", () => {
    const result = parseArgs(["--help", "init"]);
    expect(result.command).toBe("init");
    expect(result.help).toBe(true);
  });
});
