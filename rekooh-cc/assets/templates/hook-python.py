#!/usr/bin/env python3
"""Claude Code PreToolUse hook — Python template.

Exit codes:
    0 — Allow the tool call to proceed
    1 — Error (hook malfunction; fail-open by default)
    2 — Block the tool call (stderr message shown to Claude as the reason)

Stdin: JSON object with session_id, cwd, tool_name, tool_input, etc.
Stdout: Optional JSON with hookSpecificOutput for input rewriting
Stderr: Reason string when blocking (exit 2)
"""

import json
import re
import sys


def main() -> None:
    try:
        payload = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, OSError):
        sys.exit(0)  # Can't parse — fail-open

    tool_name = payload.get("tool_name", "")
    tool_input = payload.get("tool_input", {})

    # Quick exit if this isn't the tool we care about
    if tool_name != "Bash":
        sys.exit(0)

    command = tool_input.get("command", "")

    # --- Guard logic (replace with your checks) ---

    # Example: block a specific pattern
    if re.search(r"rm\s+-rf\b", command):
        print("BLOCKED: destructive command detected: rm -rf", file=sys.stderr)
        sys.exit(2)

    # --- Allow ---
    sys.exit(0)


if __name__ == "__main__":
    main()
