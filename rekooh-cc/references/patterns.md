# Hook Code Patterns

Hooks should be small, explicit programs. Prefer direct JSON parsing and visible
exit behavior over clever shell.

## Command Hook Skeleton

```bash
#!/usr/bin/env bash
set -euo pipefail

payload="$(cat)"
tool_name="$(jq -r '.tool_name // empty' <<<"$payload")"
command="$(jq -r '.tool_input.command // empty' <<<"$payload")"

if [[ "$tool_name" != "Bash" ]]; then
  exit 0
fi

if [[ "$command" == *"rm -rf"* ]]; then
  echo "Blocked: destructive rm pattern" >&2
  exit 2
fi

exit 0
```

Use Bash when the logic is tiny and the repo already assumes Unix tooling.

## Bun Hook Skeleton

```ts
#!/usr/bin/env bun

type HookInput = {
  hook_event_name: string;
  tool_name?: string;
  tool_input?: Record<string, unknown>;
};

const input = JSON.parse(await Bun.stdin.text()) as HookInput;
const command =
  typeof input.tool_input?.command === "string" ? input.tool_input.command : "";

if (input.tool_name !== "Bash") {
  process.exit(0);
}

if (/\brm\s+-rf\b/.test(command)) {
  process.stderr.write("Blocked: destructive rm pattern\n");
  process.exit(2);
}
```

Use Bun or TypeScript when parsing, path checks, or tests would be awkward in
shell.

## Practical Rules

- Read stdin exactly once.
- Parse JSON before making decisions.
- Use stderr for block reasons.
- Keep allow/block branches obvious.
- Prefer positive allowlists for high-risk guards.
- Avoid network calls in blocking hooks unless the project explicitly accepts the
  latency and failure mode.
- Put shared helper code under `.claude/hooks/lib/` only after two hooks need it.

## Anti-Patterns

- Long one-liners directly in settings
- Hook logic that mutates project files before checking event type
- Catch-all `exit 0` around the whole script
- Blocking on vague regexes that produce false positives
- Hidden dependency on a local absolute path in committed settings
- Reimplementing Claude Code's hook dispatcher inside the repo
