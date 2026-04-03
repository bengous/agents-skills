#!/usr/bin/env bash
# shellcheck disable=SC2310,SC2312
# Probes the current project for Bun MCP server installation.
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
mcp_bun_json() {
	cat <<-'JSON'
		"bun": {
		  "command": "bunx",
		  "args": ["-y", "mcp-bun@latest"],
		  "env": { "DISABLE_NOTIFICATIONS": "true" },
		  "type": "stdio"
		}
	JSON
}

# --- Case 1: No .mcp.json at all ---

if [[ ! -f "${mcp_file}" ]]; then
	cat <<XML
<bun-mcp-status installed="false" mcp-file="missing">
  <action>
    Bun MCP server is not configured (no .mcp.json found at project root).

    YOU MUST use AskUserQuestion to ask the user:
    "The Bun MCP server is not installed in this project. Want me to set it up?
     I'll create .mcp.json and you'll just need to run /reload-plugins."

    If the user accepts:
    1. Read ${mcp_file} first (it may already exist with other servers).
    2. If the file exists, merge this entry into the existing mcpServers object.
       If it does not exist, create it with this content:
       {
         "mcpServers": {
$(mcp_bun_json | indent 10)
         }
       }
    3. Tell the user to run /reload-plugins to activate the MCP server.
    4. After reload, mcp__bun__* tools become available.

    If the user declines:
    Use Context7 MCP or exa for Bun documentation lookups,
    and Bash for running bun commands directly.
  </action>
</bun-mcp-status>
XML
	exit 0
fi

# --- Case 2: .mcp.json exists, check for "bun" entry ---

has_bun=$(jq -r '.mcpServers.bun // empty' "${mcp_file}" 2>/dev/null) || true

if [[ -n "${has_bun}" ]]; then
	cat <<'XML'
<bun-mcp-status installed="true">
  <guidance>
    Bun MCP server is available. Prefer mcp__bun__* tools over Bash for:
    - Running scripts: run-bun-script-file, run-bun-eval
    - Testing: run-bun-test
    - Building: run-bun-build
    - Package management: run-bun-install
    - Performance: analyze-bun-performance, benchmark-bun-script
    Fall back to Context7/exa only for documentation lookups not covered by MCP tools.
  </guidance>
</bun-mcp-status>
XML
else
	cat <<XML
<bun-mcp-status installed="false" mcp-file="exists">
  <action>
    .mcp.json exists but has no "bun" MCP server entry.

    YOU MUST use AskUserQuestion to ask the user:
    "The Bun MCP server is not configured in this project's .mcp.json. Want me to add it?
     You'll just need to run /reload-plugins after."

    If the user accepts:
    1. Read ${mcp_file}, add "bun" to the mcpServers object:
$(mcp_bun_json | indent 7)
    2. Write the updated JSON back to ${mcp_file}.
    3. Tell the user to run /reload-plugins to activate the MCP server.

    If the user declines:
    Use Context7 MCP or exa for Bun documentation lookups,
    and Bash for running bun commands directly.
  </action>
</bun-mcp-status>
XML
fi
