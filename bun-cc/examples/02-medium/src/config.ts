// Config file handling for gh-release

import { homedir } from "node:os";
import { join } from "node:path";

// ============================================================================
// TYPES
// ============================================================================

export interface Config {
  defaultOutput?: string;
  defaultAssetPattern?: string;
}

// ============================================================================
// CONSTANTS
// ============================================================================

const CONFIG_PATH = join(homedir(), ".gh-release.json");

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function mergeWithDefaults(config: Config, overrides: { output?: string; asset?: string }) {
  return {
    output: overrides.output ?? config.defaultOutput ?? ".",
    assetPattern: overrides.asset ?? config.defaultAssetPattern,
  };
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function loadConfig(): Promise<Config> {
  if (!(await Bun.file(CONFIG_PATH).exists())) return {};
  try {
    return await Bun.file(CONFIG_PATH).json();
  } catch {
    return {};
  }
}
