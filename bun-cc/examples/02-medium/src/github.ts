// GitHub API client for gh-release

// ============================================================================
// TYPES
// ============================================================================

export interface Release {
  tag_name: string;
  assets: Asset[];
}

export interface Asset {
  name: string;
  browser_download_url: string;
  size: number;
}

export interface ParsedTarget {
  owner: string;
  repo: string;
  version?: string;
}

// ============================================================================
// CONSTANTS
// ============================================================================

const API_BASE = "https://api.github.com";
const MAX_RETRIES = 3;
const BASE_DELAY_MS = 1000;

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function parseTarget(input: string): ParsedTarget | null {
  const match = input.match(/^([^/]+)\/([^@]+)(?:@(.+))?$/);
  if (!match?.[1] || !match[2]) return null;
  const result: ParsedTarget = { owner: match[1], repo: match[2] };
  if (match[3]) result.version = match[3];
  return result;
}

export function matchAsset(assets: Asset[], pattern?: string): Asset | null {
  if (!pattern) return assets[0] ?? null;
  const regex = new RegExp(pattern.replace(/\*/g, ".*"));
  return assets.find((a) => regex.test(a.name)) ?? null;
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

async function fetchWithRetry(url: string): Promise<Response> {
  for (let i = 0; i < MAX_RETRIES; i++) {
    const res = await fetch(url, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (res.ok) return res;
    if (res.status === 404) throw new Error("Not found");
    if ((res.status === 403 || res.status >= 500) && i < MAX_RETRIES - 1) {
      await Bun.sleep(BASE_DELAY_MS * 2 ** i);
      continue;
    }
    throw new Error(`GitHub API error: ${res.status}`);
  }
  throw new Error("Max retries exceeded");
}

export async function fetchRelease(owner: string, repo: string, version?: string): Promise<Release> {
  const path = version ? `releases/tags/${version}` : "releases/latest";
  return (await fetchWithRetry(`${API_BASE}/repos/${owner}/${repo}/${path}`)).json() as Promise<Release>;
}
