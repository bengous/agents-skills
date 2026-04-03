import { join } from "node:path";
import { parseArgs } from "node:util";
import { die, printJson } from "./_lib/output.ts";
import { type HookEntry, mergeHookConfig, readSettings, writeSettings } from "./_lib/settings.ts";

const { values } = parseArgs({
  options: {
    event: { type: "string" },
    matcher: { type: "string" },
    command: { type: "string" },
    timeout: { type: "string", default: "10" },
    settings: { type: "string" },
  },
  strict: true,
});

const event = values.event ?? die("--event is required");
const command = values.command ?? die("--command is required");

const settingsPath = values.settings ?? join(process.cwd(), ".claude", "settings.json");

const before = await readSettings(settingsPath);

const hookEntry: HookEntry = {
  type: "command",
  command,
};

const timeout = Number.parseInt(values.timeout ?? "10", 10);
if (timeout !== 10) {
  hookEntry.timeout = timeout;
}

const after = mergeHookConfig(before, event, values.matcher, hookEntry);

await writeSettings(settingsPath, after);

printJson({
  settingsPath,
  event,
  matcher: values.matcher ?? null,
  command,
  timeout,
  hooksBefore: Object.keys(before.hooks ?? {}),
  hooksAfter: Object.keys(after.hooks ?? {}),
});
