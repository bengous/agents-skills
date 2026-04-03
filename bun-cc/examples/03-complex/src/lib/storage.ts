// Local file-based storage for encrypted .env files

import { readdir } from "node:fs/promises";
import { join } from "node:path";
import { type ProjectMeta, getConfigDir, getProjectDir } from "./config";
import { decrypt, encrypt } from "./crypto";

// ============================================================================
// CONSTANTS
// ============================================================================

const ENV_FILE = "env.enc";
const PROJECTS_DIR = join(getConfigDir(), "projects");

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function saveEncryptedEnv(projectPath: string, content: string, password: string): Promise<void> {
  const encrypted = await encrypt(content, password);
  await Bun.write(join(getProjectDir(projectPath), ENV_FILE), encrypted);
}

export async function loadEncryptedEnv(projectPath: string, password: string): Promise<string> {
  const file = Bun.file(join(getProjectDir(projectPath), ENV_FILE));
  if (!(await file.exists())) {
    throw new Error("No encrypted .env found. Run 'env-sync push' first.");
  }
  return decrypt(await file.bytes(), password);
}

export async function hasStoredEnv(projectPath: string): Promise<boolean> {
  return Bun.file(join(getProjectDir(projectPath), ENV_FILE)).exists();
}

export async function listProjects(): Promise<Array<{ hash: string; meta: ProjectMeta }>> {
  let entries: string[];
  try {
    entries = await readdir(PROJECTS_DIR);
  } catch {
    return [];
  }
  const projects: Array<{ hash: string; meta: ProjectMeta }> = [];
  for (const hash of entries) {
    const metaFile = Bun.file(join(PROJECTS_DIR, hash, "meta.json"));
    if (await metaFile.exists()) {
      try {
        const meta = (await metaFile.json()) as ProjectMeta;
        projects.push({ hash, meta });
      } catch {
        // Skip invalid meta files
      }
    }
  }
  return projects;
}
