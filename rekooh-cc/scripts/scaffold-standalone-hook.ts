import { chmodSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { die, printJson } from "./_lib/output.ts";

const { values } = parseArgs({
  options: {
    event: { type: "string" },
    lang: { type: "string" },
    matcher: { type: "string" },
    name: { type: "string" },
    dir: { type: "string", default: process.cwd() },
  },
  strict: true,
});

const event = values.event ?? die("--event is required");
const lang = values.lang ?? die("--lang is required");
const hookName = values.name ?? die("--name is required");
const extMap: Record<string, string> = {
  bash: "sh",
  python: "py",
  bun: "ts",
};
const templateMap: Record<string, string> = {
  bash: "hook-bash.sh",
  python: "hook-python.py",
  bun: "hook-bun-standalone.ts",
};

const ext = extMap[lang] ?? die("--lang must be bash, python, or bun");
const templateName = templateMap[lang] ?? die("--lang must be bash, python, or bun");

const skillRoot = join(process.env["HOME"] ?? "", ".claude", "skills", "rekooh");
const templatePath = join(skillRoot, "assets", "templates", templateName);

if (!existsSync(templatePath)) {
  die(`Template not found: ${templatePath}`);
}

let content = readFileSync(templatePath, "utf-8");

content = content.replace(/PreToolUse/g, event);
if (values.matcher !== undefined) {
  content = content.replace(/"Bash"/g, `"${values.matcher}"`);
  content = content.replace(/'Bash'/g, `'${values.matcher}'`);
}

const hooksDir = join(values.dir ?? process.cwd(), ".claude", "hooks");
mkdirSync(hooksDir, { recursive: true });

const outputPath = join(hooksDir, `${hookName}.${ext}`);
await Bun.write(outputPath, content);

if (lang === "bash" || lang === "python") {
  chmodSync(outputPath, 0o755);
}

printJson({
  file: outputPath,
  event,
  lang,
  matcher: values.matcher ?? null,
  name: hookName,
});
