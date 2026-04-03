// Configuration management for env-sync

import { createHash } from "node:crypto";
import { mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

// ============================================================================
// TYPES
// ============================================================================

export interface GlobalConfig {
  remote?: string;
}

export interface ProjectMeta {
  name: string;
  path: string;
  createdAt: string;
}

// ============================================================================
// CONSTANTS
// ============================================================================

const CONFIG_DIR = join(homedir(), ".env-sync");
const PROJECTS_DIR = join(CONFIG_DIR, "projects");
const CONFIG_FILE = join(CONFIG_DIR, "config.json");

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function hashPath(path: string): string {
  return createHash("sha256").update(path).digest("hex").slice(0, 16);
}

export const getConfigDir = () => CONFIG_DIR;
export const getProjectDir = (projectPath: string) => join(PROJECTS_DIR, hashPath(projectPath));

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function ensureConfigDir(): Promise<void> {
  await mkdir(PROJECTS_DIR, { recursive: true });
}

export async function loadGlobalConfig(): Promise<GlobalConfig> {
  const file = Bun.file(CONFIG_FILE);
  if (!(await file.exists())) return {};
  try {
    return await file.json();
  } catch {
    return {};
  }
}

export async function saveGlobalConfig(config: GlobalConfig): Promise<void> {
  await ensureConfigDir();
  await Bun.write(CONFIG_FILE, JSON.stringify(config, null, 2));
}

export async function loadProjectMeta(projectPath: string): Promise<ProjectMeta | null> {
  const file = Bun.file(join(getProjectDir(projectPath), "meta.json"));
  if (!(await file.exists())) return null;
  try {
    return await file.json();
  } catch {
    return null;
  }
}

export async function saveProjectMeta(projectPath: string, meta: ProjectMeta): Promise<void> {
  const dir = getProjectDir(projectPath);
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "meta.json"), JSON.stringify(meta, null, 2));
}

export async function projectExists(projectPath: string): Promise<boolean> {
  return Bun.file(join(getProjectDir(projectPath), "meta.json")).exists();
}
