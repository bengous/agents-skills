# Hook And Install Notes

The hook is optional. The skill still works manually with:

```bash
itw init <root> <initial intention>
itw get <root>
```

## Codex UserPromptSubmit

The package exposes:

```bash
itw-codex-user-prompt-submit
```

It reads JSON or plain text from stdin, detects prompts beginning with
`$intent-to-workflow`, and initializes:

```text
itw/YYYY-MM-DD-<session-short>/
```

It preserves everything after the skill token as the raw initial intention after
outer trim. Empty invocations do not create a root; the hook tells the agent to
ask for an explicit initial intention.

Example:

```text
$intent-to-workflow create a local PRD-to-workflow planner because ...
```

Mentions later in the prompt do not trigger the hook.

## Local Source Usage

From the source package:

```bash
uv run itw --help
uv run itw init /tmp/itw-demo build X because Y
uv run itw get /tmp/itw-demo
uv run itw status /tmp/itw-demo
```

## Global Install

Do not install globally unless the human asks. A typical local install is:

```bash
uv tool install /home/b3ngous/projects/agents-skills/intent-to-workflow --force
```

After install, verify:

```bash
itw --help
itw-codex-user-prompt-submit --help
```
