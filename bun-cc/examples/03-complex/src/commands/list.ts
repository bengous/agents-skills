// List all synced projects

import type { Log } from "../index";
import { listProjects } from "../lib/storage";

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function list(log: Log): Promise<number> {
  const projects = await listProjects();

  if (!projects.length) {
    log.info("No projects initialized yet.");
    return 0;
  }

  console.log("\nSynced projects:\n");
  for (const { meta } of projects) {
    console.log(`  ${meta.name}\n    Path: ${meta.path}\n    Created: ${meta.createdAt}\n`);
  }
  return 0;
}
