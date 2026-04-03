// Pull and decrypt .env from storage

import { resolve } from "node:path";
import type { Log } from "../index";
import { projectExists } from "../lib/config";
import { confirm, promptPassword } from "../lib/prompt";
import { loadEncryptedEnv } from "../lib/storage";

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function pull(log: Log): Promise<number> {
  const projectPath = resolve(process.cwd());

  if (!(await projectExists(projectPath))) {
    log.error("Project not initialized. Run 'env-sync init' first.");
    return 1;
  }

  if ((await Bun.file(".env").exists()) && !(await confirm("Overwrite existing .env?"))) {
    log.info("Aborted.");
    return 0;
  }

  const password = await promptPassword("Enter decryption password: ");
  if (!password) {
    log.error("Password cannot be empty.");
    return 1;
  }

  try {
    const content = await loadEncryptedEnv(projectPath, password);
    await Bun.write(".env", content);
    log.success("Decrypted .env written successfully.");
    return 0;
  } catch (e) {
    log.error(e instanceof Error ? e.message : "Decryption failed. Wrong password?");
    return 1;
  }
}
