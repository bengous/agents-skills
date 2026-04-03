// Initialize env-sync in current project

import { basename, resolve } from "node:path";
import { $ } from "bun";
import type { Log } from "../index";
import { projectExists, saveProjectMeta } from "../lib/config";

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function getProjectName(cwd: string): string {
  return basename(cwd);
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

async function isGitRepo(): Promise<boolean> {
  const { exitCode } = await $`git rev-parse --git-dir`.nothrow().quiet();
  return exitCode === 0;
}

export async function init(log: Log): Promise<number> {
  const projectPath = resolve(process.cwd());

  if (!(await isGitRepo())) {
    log.warning("Not a git repository. Initializing anyway.");
  }

  if (await projectExists(projectPath)) {
    log.error("Project already initialized. Use 'push' to update.");
    return 1;
  }

  await saveProjectMeta(projectPath, {
    name: getProjectName(projectPath),
    path: projectPath,
    createdAt: new Date().toISOString(),
  });

  log.success(`Initialized env-sync for '${getProjectName(projectPath)}'`);
  log.info("Run 'env-sync push' to encrypt and store your .env");
  return 0;
}
