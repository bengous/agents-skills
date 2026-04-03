import { cpSync, existsSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { die, printJson } from "./_lib/output.ts";

const { values } = parseArgs({
  options: {
    dir: { type: "string", default: process.cwd() },
    opinionated: { type: "boolean", default: false },
  },
  strict: true,
});

const projectRoot = values.dir ?? process.cwd();
const hooksDir = join(projectRoot, ".claude", "hooks");

mkdirSync(hooksDir, { recursive: true });

const created: string[] = [hooksDir];

if (values.opinionated === true) {
  const scaffoldDir = join(
    process.env["HOME"] ?? "",
    ".claude",
    "skills",
    "rekooh",
    "assets",
    "scaffolds",
    "opinionated",
  );

  if (!existsSync(scaffoldDir)) {
    die(`Scaffold directory not found: ${scaffoldDir}`);
  }

  for (const file of readdirSync(scaffoldDir)) {
    const src = join(scaffoldDir, file);
    const dest = join(hooksDir, file);
    cpSync(src, dest, { recursive: true });
    created.push(dest);
  }

  const srcDir = join(hooksDir, "src");
  mkdirSync(srcDir, { recursive: true });
  created.push(srcDir);
}

printJson({
  hooksDir,
  opinionated: values.opinionated === true,
  created,
});
