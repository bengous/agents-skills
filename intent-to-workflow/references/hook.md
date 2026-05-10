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

From an installed skill copy without a global `itw` command, call the launcher
by its installed skill path:

```bash
"<installed-skill-dir>/scripts/itw" --help
"<installed-skill-dir>/scripts/itw" init /tmp/itw-demo build X because Y
"<installed-skill-dir>/scripts/itw" get /tmp/itw-demo
"<installed-skill-dir>/scripts/itw" status /tmp/itw-demo
```

On Windows PowerShell:

```powershell
& "<installed-skill-dir>\scripts\itw.ps1" --help
& "<installed-skill-dir>\scripts\itw.ps1" init C:\Temp\itw-demo "build X because Y"
& "<installed-skill-dir>\scripts\itw.ps1" get C:\Temp\itw-demo
& "<installed-skill-dir>\scripts\itw.ps1" status C:\Temp\itw-demo
```

## Global Install

Global install is optional convenience, not required by the skill launcher.
Do not install globally unless the human asks. A typical local install is:

```bash
uv tool install /home/b3ngous/projects/agents-skills/intent-to-workflow --force
```

After install, verify:

```bash
itw --help
itw-codex-user-prompt-submit --help
```
