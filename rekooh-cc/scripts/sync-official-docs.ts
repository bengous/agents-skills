import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { printJson } from "./_lib/output.ts";

const skillRoot = join(process.env["HOME"] ?? "", ".claude", "skills", "rekooh");
const outputDir = join(skillRoot, "references", "upstream");
mkdirSync(outputDir, { recursive: true });

const sources: Array<{ url: string; filename: string }> = [
  {
    url: "https://code.claude.com/docs/en/hooks.md",
    filename: "hooks-reference.md",
  },
  {
    url: "https://code.claude.com/docs/en/hooks-guide.md",
    filename: "hooks-guide.md",
  },
  {
    url: "https://docs.claude.com/en/docs/claude-code/settings",
    filename: "settings-reference.md",
  },
];

const stripHtml = (html: string): string => {
  let text = html;

  // Preserve code blocks
  text = text.replace(/<pre[^>]*><code[^>]*>([\s\S]*?)<\/code><\/pre>/gi, (_match, code: string) => {
    const decoded = code
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&amp;/g, "&")
      .replace(/&quot;/g, '"');
    return `\n\`\`\`\n${decoded}\n\`\`\`\n`;
  });

  // Inline code
  text = text.replace(/<code[^>]*>([\s\S]*?)<\/code>/gi, "`$1`");

  // Headings
  text = text.replace(/<h1[^>]*>([\s\S]*?)<\/h1>/gi, "\n# $1\n");
  text = text.replace(/<h2[^>]*>([\s\S]*?)<\/h2>/gi, "\n## $1\n");
  text = text.replace(/<h3[^>]*>([\s\S]*?)<\/h3>/gi, "\n### $1\n");
  text = text.replace(/<h4[^>]*>([\s\S]*?)<\/h4>/gi, "\n#### $1\n");

  // List items
  text = text.replace(/<li[^>]*>([\s\S]*?)<\/li>/gi, "- $1\n");

  // Paragraphs and breaks
  text = text.replace(/<\/p>/gi, "\n\n");
  text = text.replace(/<br\s*\/?>/gi, "\n");

  // Strip remaining tags
  text = text.replace(/<[^>]+>/g, "");

  // Decode entities
  text = text.replace(/&lt;/g, "<");
  text = text.replace(/&gt;/g, ">");
  text = text.replace(/&amp;/g, "&");
  text = text.replace(/&quot;/g, '"');
  text = text.replace(/&#39;/g, "'");
  text = text.replace(/&nbsp;/g, " ");

  // Clean up whitespace
  text = text.replace(/\n{3,}/g, "\n\n");
  text = text.trim();

  return text;
};

const results: Array<{
  url: string;
  file: string;
  status: "ok" | "error";
  contentType?: string;
  error?: string;
}> = [];

for (const source of sources) {
  const outputPath = join(outputDir, source.filename);
  try {
    const response = await fetch(source.url);
    if (!response.ok) {
      results.push({
        url: source.url,
        file: outputPath,
        status: "error",
        error: `HTTP ${String(response.status)}`,
      });
      continue;
    }
    const contentType = response.headers.get("content-type") ?? "";
    const body = await response.text();
    const isMarkdown = contentType.includes("text/markdown");
    const markdown = isMarkdown ? body : stripHtml(body);
    const header = `<!-- synced: ${new Date().toISOString()} | source: ${source.url} -->\n\n`;
    await Bun.write(outputPath, `${header}${markdown}\n`);
    results.push({
      url: source.url,
      file: outputPath,
      status: "ok",
      contentType,
    });
  } catch (err) {
    results.push({
      url: source.url,
      file: outputPath,
      status: "error",
      error: err instanceof Error ? err.message : "Unknown error",
    });
  }
}

const lastSyncedPath = join(outputDir, ".last-synced");
await Bun.write(lastSyncedPath, new Date().toISOString());

printJson({ synced: new Date().toISOString(), results });
