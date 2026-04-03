// Show differences between local .env and stored version

import { resolve } from "node:path";
import type { Log } from "../index";
import { projectExists } from "../lib/config";
import { promptPassword } from "../lib/prompt";
import { hasStoredEnv, loadEncryptedEnv } from "../lib/storage";

// ============================================================================
// CONSTANTS
// ============================================================================

const ENV_FILE = ".env";
const c = { r: "\x1b[0;31m", g: "\x1b[0;32m", y: "\x1b[1;33m", x: "\x1b[0m" };

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function parseEnvFile(content: string): Map<string, string> {
  const vars = new Map<string, string>();
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const idx = trimmed.indexOf("=");
    if (idx > 0) vars.set(trimmed.slice(0, idx), trimmed.slice(idx + 1));
  }
  return vars;
}

export function computeDiff(
  local: Map<string, string>,
  stored: Map<string, string>,
): { added: string[]; removed: string[]; changed: string[] } {
  const added: string[] = [];
  const removed: string[] = [];
  const changed: string[] = [];
  for (const key of local.keys()) {
    if (!stored.has(key)) added.push(key);
    else if (local.get(key) !== stored.get(key)) changed.push(key);
  }
  for (const key of stored.keys()) {
    if (!local.has(key)) removed.push(key);
  }
  return { added, removed, changed };
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function diff(log: Log): Promise<number> {
  const projectPath = resolve(process.cwd());

  if (!(await projectExists(projectPath))) {
    log.error("Project not initialized. Run 'env-sync init' first.");
    return 1;
  }
  if (!(await hasStoredEnv(projectPath))) {
    log.error("No stored .env found. Run 'env-sync push' first.");
    return 1;
  }
  if (!(await Bun.file(ENV_FILE).exists())) {
    log.error("No local .env file found.");
    return 1;
  }

  const password = await promptPassword("Enter decryption password: ");
  if (!password) {
    log.error("Password cannot be empty.");
    return 1;
  }

  try {
    const storedContent = await loadEncryptedEnv(projectPath, password);
    const localContent = await Bun.file(ENV_FILE).text();
    const { added, removed, changed } = computeDiff(parseEnvFile(localContent), parseEnvFile(storedContent));

    if (!added.length && !removed.length && !changed.length) {
      log.success("No differences found.");
      return 0;
    }
    for (const key of added) console.log(`${c.g}+ ${key}${c.x}`);
    for (const key of removed) console.log(`${c.r}- ${key}${c.x}`);
    for (const key of changed) console.log(`${c.y}~ ${key}${c.x} (value changed)`);
    return 0;
  } catch (e) {
    log.error(e instanceof Error ? e.message : "Decryption failed. Wrong password?");
    return 1;
  }
}
