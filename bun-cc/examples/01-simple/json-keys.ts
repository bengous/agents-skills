#!/usr/bin/env bun

// Extract keys from JSON files with optional depth limiting and filtering

// ============================================================================
// CONSTANTS
// ============================================================================

const colors = {
  red: "\x1b[0;31m",
  reset: "\x1b[0m",
} as const;

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function extractKeys(obj: unknown, maxDepth = -1, filter?: RegExp, prefix = "", depth = 0): string[] {
  if (obj === null || typeof obj !== "object" || Array.isArray(obj)) return [];
  if (maxDepth !== -1 && depth >= maxDepth) return [];

  const keys: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (!filter || filter.test(fullKey)) keys.push(fullKey);
    keys.push(...extractKeys(value, maxDepth, filter, fullKey, depth + 1));
  }
  return keys;
}

export function parseArgs(args: string[]): {
  file?: string;
  count: boolean;
  depth: number;
  filter?: string;
} {
  const result: {
    file?: string;
    count: boolean;
    depth: number;
    filter?: string;
  } = {
    count: false,
    depth: -1,
  };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (!arg) continue;
    if (arg === "--count") result.count = true;
    else if (arg === "--depth") result.depth = Number.parseInt(args[++i] || "0", 10);
    else if (arg === "--filter") {
      const val = args[++i];
      if (val) result.filter = val;
    } else if (!arg.startsWith("-")) result.file = arg;
  }
  return result;
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

function error(msg: string): never {
  console.error(`${colors.red}Error:${colors.reset} ${msg}`);
  process.exit(1);
}

async function main(): Promise<void> {
  const { file, count, depth, filter } = parseArgs(Bun.argv.slice(2));

  if (!file) {
    console.error("Usage: json-keys <file.json> [--count] [--depth N] [--filter regex]");
    process.exit(1);
  }

  if (!(await Bun.file(file).exists())) {
    error(`File not found: ${file}`);
  }

  let data: unknown;
  try {
    data = await Bun.file(file).json();
  } catch {
    error(`Invalid JSON in ${file}`);
  }

  let regex: RegExp | undefined;
  if (filter) {
    try {
      regex = new RegExp(filter);
    } catch {
      error(`Invalid regex: ${filter}`);
    }
  }

  const keys = extractKeys(data, depth, regex);
  console.log(count ? keys.length : keys.join("\n"));
}

// CLI entry point (only runs when executed directly)
if (import.meta.main) {
  main();
}
