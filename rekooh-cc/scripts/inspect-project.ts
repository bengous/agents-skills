import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { die, printJson } from "./_lib/output.ts";
import { detectPackageManagers, detectQualityCommands, detectRuntimes, findProjectRoot } from "./_lib/project.ts";
import type { Settings } from "./_lib/settings.ts";

const { values } = parseArgs({
  options: {
    dir: { type: "string", default: process.cwd() },
  },
  strict: true,
});

const projectRoot = findProjectRoot(values.dir) ?? die("No project root found");

const claudeDir = join(projectRoot, ".claude");
const hooksDir = join(projectRoot, ".claude", "hooks");
const settingsPath = join(projectRoot, ".claude", "settings.json");

const hasSettingsJson = existsSync(settingsPath);

let existingHooks: string[] = [];
if (hasSettingsJson) {
  try {
    const settings = JSON.parse(readFileSync(settingsPath, "utf-8")) as Settings;
    if (settings.hooks !== undefined) {
      existingHooks = Object.keys(settings.hooks);
    }
  } catch {
    // ignore parse errors
  }
}

const runtimes = detectRuntimes(projectRoot);
const packageManagers = detectPackageManagers(projectRoot);
const qualityCommands = detectQualityCommands(projectRoot);

const hasBun = runtimes.includes("bun");
const hasTsConfig = existsSync(join(projectRoot, "tsconfig.json"));
const recommendedStrategy = hasBun || hasTsConfig ? "opinionated" : "standalone";
const rationale =
  recommendedStrategy === "opinionated"
    ? `Project uses ${[hasBun && "Bun runtime", hasTsConfig && "TypeScript"].filter(Boolean).join(" and ")} — typed runtime provides type safety and testability.`
    : "No Bun or TypeScript detected — standalone hooks are simpler with no dependencies.";

printJson({
  projectRoot,
  hasClaudeDir: existsSync(claudeDir),
  hasHooksDir: existsSync(hooksDir),
  hasSettingsJson,
  existingHooks,
  runtimes,
  packageManagers,
  qualityCommands,
  recommendedStrategy,
  rationale,
});
