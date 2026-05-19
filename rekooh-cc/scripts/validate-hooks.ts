#!/usr/bin/env bun

import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";

type JsonObject = Record<string, unknown>;

const knownHandlerTypes = new Set(["command", "http", "mcp_tool", "prompt", "agent"]);
const commandInterpreters = new Set(["bash", "sh", "zsh", "python", "python3", "node", "bun"]);

function problem(kind: "error" | "warning", path: string, message: string): JsonObject {
  return { kind, path, message };
}

function splitCommand(command: string): string[] {
  const result: string[] = [];
  let current = "";
  let quote: "'" | '"' | null = null;
  for (let index = 0; index < command.length; index += 1) {
    const char = command[index];
    if (quote !== null) {
      if (char === quote) {
        quote = null;
      } else {
        current += char;
      }
      continue;
    }
    if (char === "'" || char === '"') {
      quote = char;
      continue;
    }
    if (/\s/.test(char ?? "")) {
      if (current.length > 0) {
        result.push(current);
        current = "";
      }
      continue;
    }
    current += char;
  }
  if (current.length > 0) result.push(current);
  return result;
}

function localScriptFromCommand(command: string): string | null {
  const parts = splitCommand(command);
  if (parts.length === 0) return null;
  const candidate = commandInterpreters.has(parts[0] ?? "") ? parts[1] : parts[0];
  if (candidate === undefined) return null;
  if (candidate.startsWith("./") || candidate.startsWith(".claude/") || candidate.startsWith("/")) {
    return candidate;
  }
  return null;
}

function validateHandler(handlerPath: string, handler: unknown, projectRoot: string, issues: JsonObject[]): void {
  if (handler === null || typeof handler !== "object" || Array.isArray(handler)) {
    issues.push(problem("error", handlerPath, "handler must be an object"));
    return;
  }

  const handlerObject = handler as JsonObject;
  if (typeof handlerObject.type !== "string") {
    issues.push(problem("error", `${handlerPath}.type`, "handler type must be a string"));
    return;
  }

  if (!knownHandlerTypes.has(handlerObject.type)) {
    issues.push(problem("warning", `${handlerPath}.type`, "unknown handler type; check official docs"));
  }

  if (handlerObject.timeout !== undefined) {
    if (typeof handlerObject.timeout !== "number" || handlerObject.timeout <= 0) {
      issues.push(problem("error", `${handlerPath}.timeout`, "timeout must be a positive number of seconds"));
    }
  }

  if (handlerObject.type !== "command") {
    issues.push(problem("warning", handlerPath, "non-command handler: validate exact schema against official docs"));
    return;
  }

  if (typeof handlerObject.command !== "string" || handlerObject.command.trim() === "") {
    issues.push(problem("error", `${handlerPath}.command`, "command handler requires a non-empty command"));
    return;
  }

  if (handlerObject.args !== undefined) {
    if (!Array.isArray(handlerObject.args) || handlerObject.args.some((item) => typeof item !== "string")) {
      issues.push(problem("error", `${handlerPath}.args`, "args must be a string array when present"));
    }
  }

  const localScript = localScriptFromCommand(handlerObject.command);
  if (localScript === null) return;

  const absolute = resolve(projectRoot, localScript);
  if (!existsSync(absolute)) {
    issues.push(problem("error", `${handlerPath}.command`, `local script not found: ${absolute}`));
    return;
  }

  const mode = statSync(absolute).mode;
  if ((mode & 0o111) === 0) {
    issues.push(problem("warning", `${handlerPath}.command`, `local script is not executable: ${absolute}`));
  }
}

function validateGroup(groupPath: string, group: unknown, projectRoot: string, issues: JsonObject[]): void {
  if (group === null || typeof group !== "object" || Array.isArray(group)) {
    issues.push(problem("error", groupPath, "hook group must be an object"));
    return;
  }

  const groupObject = group as JsonObject;
  if (groupObject.matcher !== undefined && typeof groupObject.matcher !== "string") {
    issues.push(problem("error", `${groupPath}.matcher`, "matcher must be a string when present"));
  }

  if (!Array.isArray(groupObject.hooks)) {
    issues.push(problem("error", `${groupPath}.hooks`, "hooks must be an array"));
    return;
  }

  groupObject.hooks.forEach((handler, handlerIndex) => {
    validateHandler(`${groupPath}.hooks[${handlerIndex}]`, handler, projectRoot, issues);
  });
}

function validateHooks(settingsPath: string, hooks: unknown, projectRoot: string, issues: JsonObject[]): void {
  if (hooks === undefined) {
    issues.push(problem("warning", settingsPath, "no top-level hooks object"));
    return;
  }

  if (hooks === null || typeof hooks !== "object" || Array.isArray(hooks)) {
    issues.push(problem("error", settingsPath, "hooks must be an object"));
    return;
  }

  for (const [event, groups] of Object.entries(hooks as JsonObject)) {
    const eventPath = `${settingsPath}:hooks.${event}`;
    if (!Array.isArray(groups)) {
      issues.push(problem("error", eventPath, "event value must be an array of hook groups"));
      continue;
    }

    groups.forEach((group, groupIndex) => {
      validateGroup(`${eventPath}[${groupIndex}]`, group, projectRoot, issues);
    });
  }
}

const { values } = parseArgs({
  options: {
    settings: { type: "string", default: resolve(process.cwd(), ".claude", "settings.json") },
  },
  strict: true,
});

const settingsPath = resolve(values.settings ?? ".claude/settings.json");
const projectRoot = dirname(dirname(settingsPath));
const issues: JsonObject[] = [];

if (!existsSync(settingsPath)) {
  issues.push(problem("error", settingsPath, "settings file does not exist"));
} else {
  let settings: JsonObject | null = null;
  try {
    settings = JSON.parse(readFileSync(settingsPath, "utf8")) as JsonObject;
  } catch (error) {
    const detail = error instanceof Error ? error.message : "Unknown JSON parse error";
    issues.push(problem("error", settingsPath, `invalid JSON: ${detail}`));
  }

  if (settings !== null) {
    validateHooks(settingsPath, settings.hooks, projectRoot, issues);
  }
}

const errors = issues.filter((issue) => issue.kind === "error");
console.log(JSON.stringify({ settingsPath, issues }, null, 2));
process.exit(errors.length > 0 ? 1 : 0);
