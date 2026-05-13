import { describe, expect, test } from "bun:test";
import { type Asset, matchAsset, parseTarget } from "../src/github";
import { parseArgs } from "../src/index";

describe("parseTarget", () => {
  test("parses owner/repo", () => {
    expect(parseTarget("openai/codex")).toEqual({
      owner: "openai",
      repo: "codex",
      version: undefined,
    });
  });

  test("parses owner/repo@version", () => {
    expect(parseTarget("openai/codex@0.87.0")).toEqual({
      owner: "openai",
      repo: "codex",
      version: "0.87.0",
    });
  });

  test("returns null for invalid input", () => {
    expect(parseTarget("invalid")).toBeNull();
    expect(parseTarget("no-slash")).toBeNull();
    expect(parseTarget("")).toBeNull();
  });
});

describe("matchAsset", () => {
  const linuxAsset: Asset = {
    name: "app-linux.tar.gz",
    browser_download_url: "url1",
    size: 1000,
  };
  const darwinAsset: Asset = {
    name: "app-darwin.tar.gz",
    browser_download_url: "url2",
    size: 2000,
  };
  const zipAsset: Asset = {
    name: "app.zip",
    browser_download_url: "url3",
    size: 500,
  };
  const assets: Asset[] = [linuxAsset, darwinAsset, zipAsset];

  test("returns first asset when no pattern", () => {
    expect(matchAsset(assets)).toEqual(linuxAsset);
  });

  test("matches glob pattern", () => {
    expect(matchAsset(assets, "*darwin*")).toEqual(darwinAsset);
    expect(matchAsset(assets, "*.zip")).toEqual(zipAsset);
  });

  test("returns null when no match", () => {
    expect(matchAsset(assets, "*windows*")).toBeNull();
  });

  test("returns null for empty array", () => {
    expect(matchAsset([])).toBeNull();
  });
});

describe("parseArgs", () => {
  test("parses target argument", () => {
    expect(parseArgs(["openai/codex"])).toMatchObject({
      target: "openai/codex",
      list: false,
      output: ".",
    });
  });

  test("parses all flags", () => {
    const result = parseArgs([
      "org/repo",
      "--asset",
      "*.tar.gz",
      "--output",
      "./bin",
      "--list",
    ]);
    expect(result).toMatchObject({
      target: "org/repo",
      asset: "*.tar.gz",
      output: "./bin",
      list: true,
    });
  });

  test("handles --help and --version flags", () => {
    expect(parseArgs(["--help"])).toMatchObject({ help: true });
    expect(parseArgs(["-h"])).toMatchObject({ help: true });
    expect(parseArgs(["--version"])).toMatchObject({ version: true });
  });

  test("defaults when no arguments", () => {
    expect(parseArgs([])).toEqual({
      output: ".",
      list: false,
      help: false,
      version: false,
    });
  });
});
