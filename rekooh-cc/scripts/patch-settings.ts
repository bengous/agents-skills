#!/usr/bin/env bun

import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { parseArgs } from "node:util";

type JsonObject = Record<string, unknown>;

type CommandHandler = {
  type: "command";
  command: string;
  args?: string[];
  timeout?: number;
  if?: string;
  statusMessage?: string;
  async?: boolean;
  asyncRewake?: boolean;
  shell?: string;
};

type HookGroup = {
  matcher?: string;
  hooks: CommandHandler[];
};

type Settings = JsonObject & {
  hooks?: Record<string, HookGroup[]>;
};

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

function readSettings(path: string): Settings {
  if (!existsSync(path)) return {};
  try {
    return JSON.parse(readFileSync(path, "utf8")) as Settings;
  } catch (error) {
    const detail = error instanceof Error ? error.message : "Unknown JSON parse error";
    fail(`Cannot parse ${path}: ${detail}`);
  }
}

function parseArgsJson(value: string | undefined): string[] | undefined {
  if (value === undefined) return undefined;
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch (error) {
    const detail = error instanceof Error ? error.message : "Unknown JSON parse error";
    fail(`--args-json must be a JSON string array: ${detail}`);
  }
  if (!Array.isArray(parsed) || parsed.some((item) => typeof item !== "string")) {
    fail("--args-json must be a JSON string array");
  }
  return parsed;
}

function optionalNumber(value: string | undefined, name: string): number | undefined {
  if (value === undefined) return undefined;
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    fail(`${name} must be a positive number of seconds`);
  }
  return parsed;
}

function stableHandlerKey(handler: CommandHandler): string {
  return JSON.stringify({
    type: handler.type,
    command: handler.command,
    args: handler.args ?? null,
  });
}

const { values } = parseArgs({
  options: {
    settings: { type: "string", default: join(process.cwd(), ".claude", "settings.json") },
    event: { type: "string" },
    matcher: { type: "string" },
    command: { type: "string" },
    "args-json": { type: "string" },
    timeout: { type: "string" },
    if: { type: "string" },
    "status-message": { type: "string" },
    async: { type: "boolean", default: false },
    "async-rewake": { type: "boolean", default: false },
    shell: { type: "string" },
  },
  strict: true,
});

const event = values.event ?? fail("--event is required");
const command = values.command ?? fail("--command is required");
const settingsPath = values.settings ?? join(process.cwd(), ".claude", "settings.json");

const handler: CommandHandler = {
  type: "command",
  command,
};

const args = parseArgsJson(values["args-json"]);
if (args !== undefined) handler.args = args;

const timeout = optionalNumber(values.timeout, "--timeout");
if (timeout !== undefined) handler.timeout = timeout;

if (values.if !== undefined) handler.if = values.if;
if (values["status-message"] !== undefined) handler.statusMessage = values["status-message"];
if (values.async === true) handler.async = true;
if (values["async-rewake"] === true) handler.asyncRewake = true;
if (values.shell !== undefined) handler.shell = values.shell;

const settings = readSettings(settingsPath);
const hooks = { ...(settings.hooks ?? {}) };
const groups = [...(hooks[event] ?? [])];
const groupIndex = groups.findIndex((group) => group.matcher === values.matcher);

let changed = false;
if (groupIndex === -1) {
  const group: HookGroup = { hooks: [handler] };
  if (values.matcher !== undefined) group.matcher = values.matcher;
  groups.push(group);
  changed = true;
} else {
  const group = groups[groupIndex];
  if (group === undefined) fail("Internal error: missing group after lookup");
  const handlerKey = stableHandlerKey(handler);
  const alreadyExists = group.hooks.some((existing) => stableHandlerKey(existing) === handlerKey);
  if (!alreadyExists) {
    groups[groupIndex] = { ...group, hooks: [...group.hooks, handler] };
    changed = true;
  }
}

hooks[event] = groups;
const nextSettings: Settings = { ...settings, hooks };

if (changed) {
  mkdirSync(dirname(settingsPath), { recursive: true });
  await Bun.write(settingsPath, `${JSON.stringify(nextSettings, null, 2)}\n`);
}

console.log(
  JSON.stringify(
    {
      settingsPath,
      event,
      matcher: values.matcher ?? null,
      handler,
      changed,
    },
    null,
    2,
  ),
);
