#!/usr/bin/env bun

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { parseArgs } from "node:util";

type JsonObject = Record<string, unknown>;

const projectMarkers = [".git", "package.json", "Cargo.toml", "pyproject.toml", "go.mod"];

function findProjectRoot(start: string): string {
  let current = resolve(start);
  while (current !== dirname(current)) {
    if (projectMarkers.some((marker) => existsSync(join(current, marker)))) {
      return current;
    }
    current = dirname(current);
  }
  return resolve(start);
}

function readJson(path: string): { ok: true; value: JsonObject } | { ok: false; error: string } {
  try {
    return { ok: true, value: JSON.parse(readFileSync(path, "utf8")) as JsonObject };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : "Unknown JSON parse error",
    };
  }
}

function walkFiles(root: string, limit = 80): string[] {
  if (!existsSync(root)) return [];
  const found: string[] = [];
  const stack = [root];
  while (stack.length > 0 && found.length < limit) {
    const dir = stack.pop();
    if (dir === undefined) break;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) {
        stack.push(path);
      } else {
        found.push(path);
      }
      if (found.length >= limit) break;
    }
  }
  return found.sort();
}

function hookSummary(settings: JsonObject): JsonObject {
  const hooks = settings.hooks;
  if (hooks === undefined || hooks === null || typeof hooks !== "object" || Array.isArray(hooks)) {
    return { events: [], handlerTypes: [] };
  }

  const events = Object.keys(hooks as JsonObject).sort();
  const handlerTypes = new Set<string>();
  for (const groups of Object.values(hooks as JsonObject)) {
    if (!Array.isArray(groups)) continue;
    for (const group of groups) {
      if (group === null || typeof group !== "object") continue;
      const handlers = (group as JsonObject).hooks;
      if (!Array.isArray(handlers)) continue;
      for (const handler of handlers) {
        if (handler === null || typeof handler !== "object") continue;
        const type = (handler as JsonObject).type;
        if (typeof type === "string") handlerTypes.add(type);
      }
    }
  }

  return { events, handlerTypes: [...handlerTypes].sort() };
}

function packageScripts(projectRoot: string): Record<string, string> {
  const packagePath = join(projectRoot, "package.json");
  if (!existsSync(packagePath)) return {};
  const parsed = readJson(packagePath);
  if (!parsed.ok) return {};
  const scripts = parsed.value.scripts;
  if (scripts === null || typeof scripts !== "object" || Array.isArray(scripts)) return {};
  return Object.fromEntries(
    Object.entries(scripts as Record<string, unknown>).filter((entry): entry is [string, string] => {
      return typeof entry[1] === "string";
    }),
  );
}

const { values } = parseArgs({
  options: {
    dir: { type: "string", default: process.cwd() },
  },
  strict: true,
});

const projectRoot = findProjectRoot(values.dir ?? process.cwd());
const settingsPaths = [
  join(projectRoot, ".claude", "settings.json"),
  join(projectRoot, ".claude", "settings.local.json"),
  join(homedir(), ".claude", "settings.json"),
];

const settings = settingsPaths.map((path) => {
  if (!existsSync(path)) return { path, exists: false };
  const parsed = readJson(path);
  return {
    path,
    exists: true,
    parse: parsed.ok ? "ok" : "error",
    error: parsed.ok ? undefined : parsed.error,
    summary: parsed.ok ? hookSummary(parsed.value) : undefined,
  };
});

const hookFiles = walkFiles(join(projectRoot, ".claude", "hooks")).map((path) => {
  const mode = statSync(path).mode;
  return {
    path,
    executable: (mode & 0o111) !== 0,
  };
});

const scripts = packageScripts(projectRoot);

console.log(
  JSON.stringify(
    {
      projectRoot,
      settings,
      hookFiles,
      packageScripts: Object.fromEntries(
        Object.entries(scripts).filter(([name]) => ["check", "lint", "test", "typecheck", "format"].includes(name)),
      ),
      recommendations: [
        "Use project settings for shared policy and local settings for personal hooks.",
        "Run validate-hooks.ts before testing in a live Claude Code session.",
        "Check official docs for non-command handler types or unfamiliar events.",
      ],
    },
    null,
    2,
  ),
);
