# Standalone Hook Patterns

Patterns for writing hooks without the typed runtime — bash, python, or bun standalone.

## The contract

All standalone hooks:
1. Read JSON from stdin
2. Parse and inspect event-specific fields
3. Exit with the appropriate code:
   - **0** — Allow the action
   - **1** — Error (hook failed; action proceeds, warning logged)
   - **2** — Block the action
4. Write blocking reasons to **stderr**
5. Write structured responses to **stdout** (when needed)

## Bash patterns

### Read and parse stdin

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read all of stdin
INPUT=$(cat)

# Extract fields with jq
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name')
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
```

### Guard pattern (block on match)

```bash
#!/usr/bin/env bash
set -euo pipefail

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Block destructive commands
if echo "$COMMAND" | grep -qE 'rm\s+-rf\s+/|git\s+push\s+--force'; then
  echo "Blocked: destructive command detected" >&2
  exit 2
fi

exit 0
```

### Side-effect pattern (always allow, do work)

```bash
#!/usr/bin/env bash
INPUT=$(cat)
MESSAGE=$(echo "$INPUT" | jq -r '.last_assistant_message // "Session ended"')

# Send desktop notification (Linux)
notify-send "Claude" "$MESSAGE" 2>/dev/null || true

exit 0
```

### Structured response (stdout JSON)

When a hook needs to communicate more than allow/block — for example, modifying tool input or adding context:

```bash
#!/usr/bin/env bash
INPUT=$(cat)

# Add context to the session
cat <<'EOF'
{"hookSpecificOutput":{"additionalContext":"Remember: always run tests before committing"}}
EOF

exit 0
```

## Python patterns

### Read and parse stdin

```python
#!/usr/bin/env python3
import json
import sys

input_data = json.load(sys.stdin)
tool_name = input_data.get("tool_name", "")
tool_input = input_data.get("tool_input", {})
```

### Guard pattern

```python
#!/usr/bin/env python3
import json
import re
import sys

input_data = json.load(sys.stdin)
command = input_data.get("tool_input", {}).get("command", "")

DANGEROUS = [
    r"rm\s+-rf\s+/",
    r"git\s+push\s+--force",
    r":\s*>\s*/",
]

for pattern in DANGEROUS:
    if re.search(pattern, command):
        print(f"Blocked: matches dangerous pattern {pattern}", file=sys.stderr)
        sys.exit(2)

sys.exit(0)
```

### Path protection

```python
#!/usr/bin/env python3
import json
import sys

input_data = json.load(sys.stdin)
file_path = input_data.get("tool_input", {}).get("file_path", "")

PROTECTED = [".env", "credentials.json", ".claude/settings.json"]

for p in PROTECTED:
    if file_path.endswith(p) or p in file_path:
        print(f"Blocked: {file_path} is a protected path", file=sys.stderr)
        sys.exit(2)

sys.exit(0)
```

## Bun standalone patterns

### Read and parse stdin

```typescript
const input = await Bun.stdin.text();
const data = JSON.parse(input);
const toolName: string = data.tool_name;
const toolInput: Record<string, unknown> = data.tool_input;
```

### Guard pattern

```typescript
const input = await Bun.stdin.text();
const data = JSON.parse(input);
const command: string = data.tool_input?.command ?? "";

const DANGEROUS = [
  /rm\s+-rf\s+\//,
  /git\s+push\s+--force/,
  /mkfs\./,
];

for (const pattern of DANGEROUS) {
  if (pattern.test(command)) {
    process.stderr.write(`Blocked: ${pattern}\n`);
    process.exit(2);
  }
}

process.exit(0);
```

### PostToolUse pattern (run command after edit)

```typescript
const input = await Bun.stdin.text();
const data = JSON.parse(input);
const filePath: string = data.tool_input?.file_path ?? "";

if (filePath.endsWith(".ts") || filePath.endsWith(".tsx")) {
  const result = Bun.spawnSync(["npx", "biome", "check", "--write", filePath]);
  if (result.exitCode !== 0) {
    process.stderr.write(`Lint failed: ${result.stderr.toString()}\n`);
    process.exit(2);
  }
}

process.exit(0);
```

## Exit code reference

| Code | Meaning | When to use |
|------|---------|-------------|
| 0 | Allow | Action is safe, or hook is side-effect-only |
| 1 | Error | Hook itself failed (parse error, missing tool, etc.) |
| 2 | Block | Action should be prevented; write reason to stderr |
