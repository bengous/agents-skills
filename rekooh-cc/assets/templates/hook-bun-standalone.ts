#!/usr/bin/env bun

/**
 * Claude Code PreToolUse hook — standalone Bun/TypeScript template.
 *
 * No framework dependencies. Types are defined inline.
 *
 * Exit codes:
 *   0 — Allow the tool call to proceed
 *   1 — Error (hook malfunction; fail-open by default)
 *   2 — Block the tool call (stderr message shown to Claude as the reason)
 *
 * Stdin: JSON object with session_id, cwd, tool_name, tool_input, etc.
 * Stdout: Optional JSON with hookSpecificOutput for input rewriting
 * Stderr: Reason string when blocking (exit 2)
 */

// -- Inline types for common PreToolUse fields --

interface PreToolUseInput {
	session_id: string;
	cwd: string;
	tool_name: string;
	tool_input: Record<string, unknown>;
	hook_event_name: string;
	permission_mode: string;
	transcript_path: string;
}

// -- Read stdin --

const raw = await Bun.stdin.text();

let input: PreToolUseInput;
try {
	input = JSON.parse(raw) as PreToolUseInput;
} catch {
	process.exit(0); // Can't parse — fail-open
}

// Quick exit if this isn't the tool we care about
if (input.tool_name !== "Bash") {
	process.exit(0);
}

const command = (input.tool_input as { command?: string }).command ?? "";

// --- Guard logic (replace with your checks) ---

// Example: block a specific pattern
if (/rm\s+-rf\b/.test(command)) {
	process.stderr.write("BLOCKED: destructive command detected: rm -rf\n");
	process.exit(2);
}

// --- Allow ---
process.exit(0);
