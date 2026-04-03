#!/usr/bin/env bash
set -euo pipefail

# Claude Code PreToolUse hook — Bash template
#
# Exit codes:
#   0 — Allow the tool call to proceed
#   1 — Error (hook malfunction; fail-open by default)
#   2 — Block the tool call (stderr message shown to Claude as the reason)
#
# Stdin: JSON object with session_id, cwd, tool_name, tool_input, etc.
# Stdout: Optional JSON with hookSpecificOutput for input rewriting
# Stderr: Reason string when blocking (exit 2)

# Read the tool input from stdin JSON
INPUT=$(cat)
tool_name=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null) || exit 0
command=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null) || exit 0

# Quick exit if this isn't the tool we care about
[[ "$tool_name" == "Bash" ]] || exit 0

# --- Guard logic (replace with your checks) ---

# Example: block a specific pattern
if [[ "$command" =~ rm\ -rf ]]; then
    echo "BLOCKED: destructive command detected: rm -rf" >&2
    exit 2
fi

# --- Allow ---
exit 0
