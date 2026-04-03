// Download and extraction logic for gh-release

import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import type { Asset } from "./github";

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function downloadAndExtract(asset: Asset, outputDir: string): Promise<number> {
  await mkdir(outputDir, { recursive: true });
  process.stdout.write(`Downloading ${asset.name}`);
  const res = await fetch(asset.browser_download_url);
  if (!res.ok) throw new Error(`Download failed: ${res.status}`);
  const bytes = await res.bytes();
  process.stdout.write(" done\n");

  if (asset.name.endsWith(".tar.gz") || asset.name.endsWith(".tgz")) {
    process.stdout.write("Extracting...");
    const count = await new Bun.Archive(bytes).extract(outputDir);
    process.stdout.write(` ${count} files\n`);
    return count;
  }
  await Bun.write(join(outputDir, asset.name), bytes);
  return 1;
}
