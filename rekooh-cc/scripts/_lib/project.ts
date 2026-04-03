import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";

const PROJECT_MARKERS = [".git", "package.json", "Cargo.toml", "pyproject.toml", "go.mod"];

const findProjectRoot = (startDir?: string): string | null => {
  let dir = resolve(startDir ?? process.cwd());
  const root = "/";

  while (dir !== root) {
    for (const marker of PROJECT_MARKERS) {
      if (existsSync(join(dir, marker))) {
        return dir;
      }
    }
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }

  return null;
};

const findSettingsFiles = (projectRoot: string): { user: string | null; project: string | null } => {
  const userPath = join(homedir(), ".claude", "settings.json");
  const projectPath = join(projectRoot, ".claude", "settings.json");

  return {
    user: existsSync(userPath) ? userPath : null,
    project: existsSync(projectPath) ? projectPath : null,
  };
};

const RUNTIME_MAP: Record<string, string> = {
  "bun.lock": "bun",
  "bun.lockb": "bun",
  "package-lock.json": "node",
  "yarn.lock": "node",
  "pnpm-lock.yaml": "node",
  "Cargo.lock": "rust",
  "go.sum": "go",
};

const detectRuntimes = (projectRoot: string): string[] => {
  const runtimes = new Set<string>();
  for (const [lockFile, runtime] of Object.entries(RUNTIME_MAP)) {
    if (existsSync(join(projectRoot, lockFile))) {
      runtimes.add(runtime);
    }
  }
  return [...runtimes];
};

const MANAGER_MAP: Record<string, string> = {
  "bun.lock": "bun",
  "bun.lockb": "bun",
  "package-lock.json": "npm",
  "yarn.lock": "yarn",
  "pnpm-lock.yaml": "pnpm",
};

const detectPackageManagers = (projectRoot: string): string[] => {
  const managers = new Set<string>();
  for (const [lockFile, manager] of Object.entries(MANAGER_MAP)) {
    if (existsSync(join(projectRoot, lockFile))) {
      managers.add(manager);
    }
  }
  return [...managers];
};

const detectQualityCommands = (projectRoot: string): Record<string, string> => {
  const pkgPath = join(projectRoot, "package.json");
  if (!existsSync(pkgPath)) return {};

  try {
    const pkg = JSON.parse(readFileSync(pkgPath, "utf-8")) as {
      scripts?: Record<string, string>;
    };
    const scripts = pkg.scripts ?? {};
    const commands: Record<string, string> = {};
    const keys = ["lint", "format", "test", "typecheck", "check"] as const;

    for (const key of keys) {
      const value = scripts[key];
      if (value !== undefined) {
        commands[key] = value;
      }
    }

    return commands;
  } catch {
    return {};
  }
};

export { detectPackageManagers, detectQualityCommands, detectRuntimes, findProjectRoot, findSettingsFiles };
