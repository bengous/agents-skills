import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { die, printJson } from "./_lib/output.ts";

const { values } = parseArgs({
  options: {
    event: { type: "string" },
    matcher: { type: "string" },
    name: { type: "string" },
    dir: { type: "string", default: process.cwd() },
  },
  strict: true,
});

const event = values.event ?? die("--event is required");
const hookName = values.name ?? die("--name is required");

const skillRoot = join(process.env["HOME"] ?? "", ".claude", "skills", "rekooh");
const templatePath = join(skillRoot, "assets", "templates", "hook-bun-opinionated.ts");

if (!existsSync(templatePath)) {
  die(`Template not found: ${templatePath}`);
}

let content = readFileSync(templatePath, "utf-8");

if (!content.includes("{{EVENT}}") || !content.includes("{{MATCHER_LINE}}")) {
  die("Template is missing {{EVENT}} or {{MATCHER_LINE}} markers");
}

content = content.replaceAll("{{EVENT}}", event);
if (values.matcher !== undefined) {
  content = content.replace(/{{MATCHER_LINE}}/g, `\tmatcher: "${values.matcher}",`);
} else {
  content = content.replace(/{{MATCHER_LINE}}\n/g, "");
}

const hooksDir = join(values.dir ?? process.cwd(), ".claude", "hooks");
const srcDir = join(hooksDir, "src");
mkdirSync(srcDir, { recursive: true });

const outputPath = join(srcDir, `${hookName}.ts`);
await Bun.write(outputPath, content);

printJson({
  file: outputPath,
  event,
  matcher: values.matcher ?? null,
  name: hookName,
});
