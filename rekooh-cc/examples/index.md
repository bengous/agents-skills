# Examples

Complete, working hook implementations organized by use case.

| Example | Event | Strategy | Description |
|---------|-------|----------|-------------|
| [block-destructive-bash](block-destructive-bash/index.md) | PreToolUse | Bash | Block dangerous shell commands |
| [protect-paths](protect-paths/index.md) | PreToolUse | Bash | Prevent modifications to sensitive files |
| [run-lint-after-edit](run-lint-after-edit/index.md) | PostToolUse | Bash | Auto-run linter after file edits |
| [stop-notification](stop-notification/index.md) | Stop | Bash | Desktop notification when Claude stops |
| [typed-hook-with-opinionated-lib](typed-hook-with-opinionated-lib/index.md) | PreToolUse | Typed runtime | Full typed hook using defineHook |
| [complete-project-bootstrap](complete-project-bootstrap/index.md) | — | — | End-to-end walkthrough using all scripts |

## How to use examples

Each example includes:
1. **Problem statement** — what you want to achieve
2. **Solution** — complete, working implementation
3. **Settings registration** — exact JSON for settings.json
4. **Validation** — how to verify it works
