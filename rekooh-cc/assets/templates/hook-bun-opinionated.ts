#!/usr/bin/env bun

/**
 * Claude Code {{EVENT}} hook — using the typed runtime (claude-hooks).
 *
 * The defineHook() call reads stdin, decodes the event, runs your check
 * callback with typed input and response builders, renders the output,
 * and exits with the correct code. You just write the check logic.
 *
 * Response builders available for {{EVENT}}:
 *   respond.allow()           — exit 0, no output
 *   respond.deny(reason)      — exit 2, stderr = reason
 *   respond.ask(reason?)      — exit 0, prompt user for permission
 *   respond.allowWithInput(updatedInput, context?) — exit 0, rewrite tool input
 */

import { defineHook } from "claude-hooks";

defineHook("{{EVENT}}", {
{{MATCHER_LINE}}

	// failurePolicy: "fail-open" (default) or "fail-closed"
	// fail-open: if the hook errors, allow the tool call
	// fail-closed: if the hook errors, block the tool call
	failurePolicy: "fail-open",

	check: (input, respond) => {
		const command = (input.tool_input as { command?: string }).command;
		if (typeof command !== "string") return respond.allow();

		// --- Guard logic (replace with your checks) ---

		// Example: block a specific pattern
		if (/rm\s+-rf\b/.test(command)) {
			return respond.deny("BLOCKED: destructive command detected: rm -rf");
		}

		// Example: rewrite tool input
		// return respond.allowWithInput(
		//   { ...input.tool_input, command: command.replace(/--force/g, "") },
		//   "Removed --force flag"
		// );

		return respond.allow();
	},
});
