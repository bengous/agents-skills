#!/usr/bin/env bun

// env-sync: Sync .env files across machines with encryption

import { diff } from "./commands/diff";
import { init } from "./commands/init";
import { list } from "./commands/list";
import { pull } from "./commands/pull";
import { push } from "./commands/push";
import { loadGlobalConfig, saveGlobalConfig } from "./lib/config";

// ============================================================================
// TYPES
// ============================================================================

export interface Log {
  info: (msg: string) => void;
  success: (msg: string) => void;
  warning: (msg: string) => void;
  error: (msg: string) => void;
}

// ============================================================================
// CONSTANTS
// ============================================================================

const VERSION = "1.0.0";
const USAGE = `env-sync v${VERSION} - Sync .env files with encryption

Commands:
  init      Initialize in current project
  push      Encrypt and push .env
  pull      Pull and decrypt .env
  diff      Show differences
  list      List synced projects
  config    set remote <url>

Options:
  --help    Show this help
  --version Show version`;

const c = {
  r: "\x1b[0;31m",
  g: "\x1b[0;32m",
  y: "\x1b[1;33m",
  b: "\x1b[0;34m",
  x: "\x1b[0m",
};
const log: Log = {
  info: (m) => console.log(`${c.b}i${c.x} ${m}`),
  success: (m) => console.log(`${c.g}v${c.x} ${m}`),
  warning: (m) => console.log(`${c.y}!${c.x} ${m}`),
  error: (m) => console.error(`${c.r}x${c.x} ${m}`),
};

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function parseArgs(args: string[]): {
  command?: string;
  subArgs: string[];
  help: boolean;
  version: boolean;
} {
  const result: {
    command?: string;
    subArgs: string[];
    help: boolean;
    version: boolean;
  } = {
    subArgs: [],
    help: false,
    version: false,
  };
  for (const arg of args) {
    if (arg === "--help" || arg === "-h") result.help = true;
    else if (arg === "--version") result.version = true;
    else if (!arg.startsWith("-") && !result.command) result.command = arg;
    else if (result.command) result.subArgs.push(arg);
  }
  return result;
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

async function handleConfig(subArgs: string[]): Promise<number> {
  if (subArgs[0] !== "set" || subArgs[1] !== "remote" || !subArgs[2]) {
    log.error("Usage: env-sync config set remote <url>");
    return 1;
  }
  const config = await loadGlobalConfig();
  config.remote = subArgs[2];
  await saveGlobalConfig(config);
  log.success(`Remote set to: ${subArgs[2]}`);
  return 0;
}

const commands: Record<string, (log: Log) => Promise<number>> = {
  init,
  push,
  pull,
  diff,
  list,
};

async function main(): Promise<void> {
  const { command, subArgs, help, version } = parseArgs(Bun.argv.slice(2));

  if (version) {
    console.log(VERSION);
    return;
  }
  if (help || !command) {
    console.log(USAGE);
    process.exit(help ? 0 : 1);
  }

  let exitCode: number;
  if (command === "config") {
    exitCode = await handleConfig(subArgs);
  } else if (commands[command]) {
    exitCode = await commands[command](log);
  } else {
    log.error(`Unknown command: ${command}`);
    console.log(USAGE);
    exitCode = 1;
  }
  process.exit(exitCode);
}

if (import.meta.main) {
  main().catch((e) => {
    log.error(e instanceof Error ? e.message : String(e));
    process.exit(1);
  });
}
