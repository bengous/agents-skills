# Hook And Install Notes

The hook is optional. The skill still works manually with:

```bash
itw init <id>
# edit .itw/<id>/intake with the raw initial intention
itw get <id>
```

## Codex UserPromptSubmit

The package exposes:

```bash
itw-codex-user-prompt-submit
```

It reads JSON or plain text from stdin, detects prompts beginning with
`$intent-to-workflow`, derives `<id>` from the git root or cwd basename, and
initializes:

```text
.itw/<id>/
```

The hook does not write `intake`. It scaffolds the root, then tells the agent to
edit `.itw/<id>/intake` with everything after the skill token as the raw initial
intention. Empty invocations do not create a root; the hook tells the agent to
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
uv run itw init demo
uv run itw get demo
uv run itw status demo
```

From an installed skill copy without a global `itw` command, call the launcher
by its installed skill path:

```bash
"<installed-skill-dir>/scripts/itw" --help
"<installed-skill-dir>/scripts/itw" init demo
"<installed-skill-dir>/scripts/itw" get demo
"<installed-skill-dir>/scripts/itw" status demo
```

On Windows PowerShell:

```powershell
& "<installed-skill-dir>\scripts\itw.ps1" --help
& "<installed-skill-dir>\scripts\itw.ps1" init demo
& "<installed-skill-dir>\scripts\itw.ps1" get demo
& "<installed-skill-dir>\scripts\itw.ps1" status demo
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
