import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { dirname } from "node:path";

interface HookEntry {
  type: "command";
  command: string;
  timeout?: number;
}

interface HookGroup {
  matcher?: string;
  hooks: HookEntry[];
}

interface Settings {
  hooks?: Record<string, HookGroup[]>;
  [key: string]: unknown;
}

const readSettings = async (path: string): Promise<Settings> => {
  if (!existsSync(path)) {
    return {};
  }
  try {
    const content = await readFile(path, "utf-8");
    return JSON.parse(content) as Settings;
  } catch {
    return {};
  }
};

const mergeHookConfig = (
  settings: Settings,
  event: string,
  matcher: string | undefined,
  hookEntry: HookEntry,
): Settings => {
  const result = { ...settings };
  const hooks = result.hooks !== undefined ? { ...result.hooks } : {};

  const existing = hooks[event];
  const groups: HookGroup[] = existing !== undefined ? [...existing] : [];

  const groupIndex = groups.findIndex((g) => (matcher === undefined ? g.matcher === undefined : g.matcher === matcher));

  if (groupIndex === -1) {
    const newGroup: HookGroup = {
      hooks: [hookEntry],
    };
    if (matcher !== undefined) {
      newGroup.matcher = matcher;
    }
    groups.push(newGroup);
  } else {
    const existing = groups[groupIndex];
    if (existing) {
      const alreadyExists = existing.hooks.some((h) => h.command === hookEntry.command);
      if (!alreadyExists) {
        groups[groupIndex] = {
          ...existing,
          hooks: [...existing.hooks, hookEntry],
        };
      }
    }
  }

  hooks[event] = groups;
  result.hooks = hooks;
  return result;
};

const writeSettings = async (path: string, settings: Settings): Promise<void> => {
  const dir = dirname(path);
  const { mkdirSync } = await import("node:fs");
  mkdirSync(dir, { recursive: true });
  await Bun.write(path, `${JSON.stringify(settings, null, 2)}\n`);
};

export type { HookEntry, HookGroup, Settings };
export { mergeHookConfig, readSettings, writeSettings };
