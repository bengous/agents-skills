#!/usr/bin/env bun

// gh-release: Download and extract GitHub release assets

import { loadConfig, mergeWithDefaults } from "./config";
import { downloadAndExtract, formatSize } from "./download";
import { fetchRelease, matchAsset, parseTarget } from "./github";

// ============================================================================
// TYPES
// ============================================================================

interface CLIOptions {
  target?: string;
  asset?: string;
  output: string;
  list: boolean;
  help: boolean;
  version: boolean;
}

// ============================================================================
// CONSTANTS
// ============================================================================

const VERSION = "1.0.0";
const USAGE = `gh-release v${VERSION}
Usage: gh-release <owner/repo[@version]> [--asset <pattern>] [--output <dir>] [--list]

Examples:
  gh-release openai/codex                              # Latest release
  gh-release openai/codex@0.87.0 --asset "*.tar.gz"   # Specific version`;

const c = { r: "\x1b[0;31m", g: "\x1b[0;32m", b: "\x1b[0;34m", x: "\x1b[0m" };
const log = {
  info: (m: string) => console.log(`${c.b}i${c.x} ${m}`),
  success: (m: string) => console.log(`${c.g}v${c.x} ${m}`),
  error: (m: string) => console.error(`${c.r}x${c.x} ${m}`),
};

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function parseArgs(args: string[]): CLIOptions {
  const opts: CLIOptions = {
    output: ".",
    list: false,
    help: false,
    version: false,
  };
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (!a) continue;
    if (a === "--version") opts.version = true;
    else if (a === "--help" || a === "-h") opts.help = true;
    else if (a === "--list") opts.list = true;
    else if (a === "--asset") {
      const val = args[++i];
      if (val) opts.asset = val;
    } else if (a === "--output") {
      const val = args[++i];
      if (val) opts.output = val;
    } else if (!a.startsWith("-")) opts.target = a;
  }
  return opts;
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

async function main(): Promise<void> {
  const opts = parseArgs(Bun.argv.slice(2));

  if (opts.version) {
    console.log(VERSION);
    return;
  }

  if (opts.help || !opts.target) {
    console.log(USAGE);
    if (!opts.help) process.exit(1);
    return;
  }

  const parsed = parseTarget(opts.target);
  if (!parsed) {
    log.error(`Invalid target: ${opts.target} (expected owner/repo[@version])`);
    process.exit(1);
  }

  const config = await loadConfig();
  const overrides: { output?: string; asset?: string } = {
    output: opts.output,
  };
  if (opts.asset) overrides.asset = opts.asset;
  const { output, assetPattern } = mergeWithDefaults(config, overrides);

  log.info(`Fetching ${parsed.owner}/${parsed.repo}${parsed.version ? `@${parsed.version}` : " (latest)"}`);

  const release = await fetchRelease(parsed.owner, parsed.repo, parsed.version);
  log.info(`Release: ${release.tag_name}`);

  if (opts.list) {
    console.log("\nAssets:");
    for (const asset of release.assets) {
      console.log(`  ${asset.name} (${formatSize(asset.size)})`);
    }
    return;
  }

  const asset = matchAsset(release.assets, assetPattern);
  if (!asset) {
    log.error("No matching asset found");
    process.exit(1);
  }

  const count = await downloadAndExtract(asset, output);
  log.success(`Extracted ${count} file(s) to ${output}`);
}

// CLI entry point
if (import.meta.main) {
  main().catch((e) => {
    log.error(e instanceof Error ? e.message : String(e));
    process.exit(1);
  });
}
