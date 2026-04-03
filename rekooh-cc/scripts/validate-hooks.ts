import { existsSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { printJson } from "./_lib/output.ts";
import { readSettings } from "./_lib/settings.ts";

const { values } = parseArgs({
  options: {
    dir: { type: "string", default: process.cwd() },
  },
  strict: true,
});

const projectRoot = values.dir ?? process.cwd();
const settingsPath = join(projectRoot, ".claude", "settings.json");

interface ValidationResult {
  event: string;
  matcher: string | null;
  command: string;
  exists: boolean;
  executable: boolean;
}

const extractScriptPath = (command: string): string => {
  const parts = command.split(/\s+/);
  const interpreters = new Set(["bash", "sh", "bun", "node", "python", "python3"]);
  const first = parts[0] ?? "";
  if (parts.length > 1 && interpreters.has(first)) {
    return parts[1] ?? first;
  }
  return first;
};

const results: ValidationResult[] = [];

const settings = await readSettings(settingsPath);
const hooks = settings.hooks ?? {};

for (const [event, groups] of Object.entries(hooks)) {
  for (const group of groups) {
    for (const hook of group.hooks) {
      const cmdPath = resolve(projectRoot, extractScriptPath(hook.command));
      const fileExists = existsSync(cmdPath);
      let isExecutable = false;

      if (fileExists) {
        try {
          const stat = statSync(cmdPath);
          isExecutable = (stat.mode & 0o111) !== 0;
        } catch {
          // stat failed
        }
      }

      results.push({
        event,
        matcher: group.matcher ?? null,
        command: hook.command,
        exists: fileExists,
        executable: isExecutable,
      });
    }
  }
}

const hasFailures = results.some((r) => !r.exists || !r.executable);
printJson(results);
process.exit(hasFailures ? 1 : 0);
