#!/usr/bin/env bun

const links = [
  "https://code.claude.com/docs/en/hooks",
  "https://code.claude.com/docs/en/hooks-guide",
  "https://code.claude.com/docs/en/settings",
  "https://code.claude.com/docs/llms.txt",
  "https://code.claude.com/docs/en/changelog",
  "https://github.com/garrytan/gbrain",
  "https://raw.githubusercontent.com/garrytan/gbrain/master/AGENTS.md",
  "https://raw.githubusercontent.com/garrytan/gbrain/master/llms.txt",
  "https://raw.githubusercontent.com/garrytan/gbrain/master/skills/RESOLVER.md",
];

const results = await Promise.all(
  links.map(async (url) => {
    try {
      const response = await fetch(url, { method: "HEAD", redirect: "follow" });
      return {
        url,
        ok: response.ok,
        status: response.status,
        finalUrl: response.url,
      };
    } catch (error) {
      return {
        url,
        ok: false,
        error: error instanceof Error ? error.message : "Unknown fetch error",
      };
    }
  }),
);

console.log(JSON.stringify({ checkedAt: new Date().toISOString(), results }, null, 2));
process.exit(results.every((result) => result.ok) ? 0 : 1);
