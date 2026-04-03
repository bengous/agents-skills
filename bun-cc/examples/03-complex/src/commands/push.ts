// Encrypt and push .env to storage

import { resolve } from "node:path";
import type { Log } from "../index";
import { projectExists } from "../lib/config";
import { promptPassword } from "../lib/prompt";
import { saveEncryptedEnv } from "../lib/storage";

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function push(log: Log): Promise<number> {
  const projectPath = resolve(process.cwd());

  if (!(await projectExists(projectPath))) {
    log.error("Project not initialized. Run 'env-sync init' first.");
    return 1;
  }

  const envFile = Bun.file(".env");
  if (!(await envFile.exists())) {
    log.error("No .env file found in current directory.");
    return 1;
  }

  const content = await envFile.text();
  if (!content.trim()) {
    log.error(".env file is empty.");
    return 1;
  }

  const password = await promptPassword("Enter encryption password: ");
  if (!password) {
    log.error("Password cannot be empty.");
    return 1;
  }

  const confirmPwd = await promptPassword("Confirm password: ");
  if (password !== confirmPwd) {
    log.error("Passwords do not match.");
    return 1;
  }

  await saveEncryptedEnv(projectPath, content, password);
  log.success("Encrypted .env stored successfully.");
  return 0;
}
