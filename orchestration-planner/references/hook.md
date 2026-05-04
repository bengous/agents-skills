# Hook And Install Notes

The hook is optional. The skill still works manually with:

```bash
orch init <root> [intention...]
```

## Codex UserPromptSubmit

The package exposes:

```bash
orch-codex-user-prompt-submit
```

It reads JSON or plain text from stdin, detects prompts beginning with
`$orchestration-planner`, and initializes:

```text
orc/YYYY-MM-DD-<session-short>/
```

It preserves everything after the skill token as raw intention.

Examples:

```text
$orchestration-planner
$orchestration-planner create a PRD-to-workflow planner because ...
```

Mentions later in the prompt do not trigger the hook.

## Local Source Usage

From the source package:

```bash
uv run orch --help
uv run orch init /tmp/orch-demo build X because Y
uv run orch status /tmp/orch-demo
```

## Global Install

Do not install globally unless the human asks. A typical local install is:

```bash
uv tool install /home/b3ngous/projects/agents-skills/orchestration-planner --force
```

After install, verify:

```bash
orch --help
orch-codex-user-prompt-submit --help
```
