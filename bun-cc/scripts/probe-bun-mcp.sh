#!/usr/bin/env bash
# shellcheck disable=SC2310,SC2312
# Probes the current project for the official Bun docs MCP server (bun-docs).
# Called via !`command` preprocessing when the skill loads.
# Outputs XML to <bun-mcp-status>.
set -euo pipefail

# --- Dependency checks (early exit with useful output) ---

for dep in jq git; do
	command -v "${dep}" >/dev/null 2>&1 || {
		printf '<bun-mcp-status error="missing dependency: %s">\n' "${dep}"
		printf '  <action>Install %s to enable Bun MCP detection.</action>\n' "${dep}"
		printf '</bun-mcp-status>\n'
		exit 0
	}
done

# --- Locate project root and .mcp.json ---

project_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
mcp_file="${project_root}/.mcp.json"

# --- Helpers ---

# Indent each line of stdin by N spaces.
indent() {
	local n="${1:-0}"
	local pad
	pad=$(printf '%*s' "${n}" '')
	sed "s/^/${pad}/"
}

# The JSON block to inject into .mcp.json (canonical, no extra indentation).
mcp_bun_docs_json() {
	cat <<-'JSON'
		"bun-docs": {
		  "type": "http",
		  "url": "https://bun.com/docs/mcp"
		}
	JSON
}

# --- Case 1: No .mcp.json at all ---

if [[ ! -f "${mcp_file}" ]]; then
	cat <<XML
<bun-mcp-status installed="false" mcp-file="missing">
  <action>
    Bun docs MCP server is not configured (no .mcp.json found at project root).

    The Bun docs MCP server is optional. Before editing project config, ask the
    user whether to add it:
    "The Bun docs MCP server is not configured in this project. Want me to create
     .mcp.json? You will need to reload plugins/tools after."

    If the user accepts:
    1. Read ${mcp_file} first (it may already exist with other servers).
    2. If the file exists, merge this entry into the existing mcpServers object.
       If it does not exist, create it with this content:
       {
         "mcpServers": {
$(mcp_bun_docs_json | indent 10)
         }
       }
    3. Tell the user to run /reload-plugins to activate the MCP server.
    4. After reload, the official Bun docs tools become available: search_bun,
       read_bun_page, list_bun_pages, grep_bun.

    If the user declines:
    Use official Bun docs/Context7/exa for documentation lookups, and Bash for
    running bun commands directly.
  </action>
</bun-mcp-status>
XML
	exit 0
fi

# --- Case 2: .mcp.json exists, check for "bun-docs" entry ---

has_bun=$(jq -r '.mcpServers["bun-docs"] // empty' "${mcp_file}" 2>/dev/null) || true

if [[ -n "${has_bun}" ]]; then
	cat <<'XML'
<bun-mcp-status installed="true">
  <guidance>
    Bun docs MCP server is available. Prefer its official documentation tools
    over fetching pages by hand:
    - Search the docs: search_bun
    - Read a page: read_bun_page
    - List pages under a path: list_bun_pages
    - Exact keyword/regex search: grep_bun
    Run bun commands directly with Bash; fall back to Context7/exa for lookups
    not covered by these tools.
  </guidance>
</bun-mcp-status>
XML
else
	cat <<XML
<bun-mcp-status installed="false" mcp-file="exists">
  <action>
    .mcp.json exists but has no "bun-docs" MCP server entry.

    The Bun docs MCP server is optional. Before editing project config, ask the
    user whether to add it:
    "The Bun docs MCP server is not configured in this project's .mcp.json.
     Want me to add it? You will need to reload plugins/tools after."

    If the user accepts:
    1. Read ${mcp_file}, add "bun-docs" to the mcpServers object:
$(mcp_bun_docs_json | indent 7)
    2. Write the updated JSON back to ${mcp_file}.
    3. Tell the user to run /reload-plugins to activate the MCP server.

    If the user declines:
    Use official Bun docs/Context7/exa for documentation lookups, and Bash for
    running bun commands directly.
  </action>
</bun-mcp-status>
XML
fi
