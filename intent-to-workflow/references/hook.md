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
uv run itw setup status
uv run itw --help
uv run itw init demo
uv run itw get demo
uv run itw status demo
```

From an installed skill copy without a global `itw` command, call the launcher
by its installed skill path:

```bash
"<installed-skill-dir>/scripts/itw" setup status
"<installed-skill-dir>/scripts/itw" --help
"<installed-skill-dir>/scripts/itw" init demo
"<installed-skill-dir>/scripts/itw" get demo
"<installed-skill-dir>/scripts/itw" status demo
```

On Windows PowerShell:

```powershell
& "<installed-skill-dir>\scripts\itw.ps1" setup status
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
itw setup status
itw --help
itw-codex-user-prompt-submit --help
```

## Codex Agent Types

The public source for the Codex subagent personas used by generated workflows
lives in:

```text
intent-to-workflow/src/intent_to_workflow/agents/codex/itw-*.toml
```

Those files are the role config layers. Codex agent types are registered from
`config.toml` with `agents.<name>.description` and
`agents.<name>.config_file`. See:

```text
intent-to-workflow/src/intent_to_workflow/agents/codex/config.example.toml
```

Use the single setup gate:

```bash
itw setup status
```

If it fails and the human wants to install the runtime setup:

```bash
"<installed-skill-dir>/scripts/setup"
itw setup status
```

On Windows PowerShell:

```powershell
& "<installed-skill-dir>\scripts\setup.ps1"
itw setup status
```

The setup script installs the global `itw` command, copies packaged
`itw-*.toml` config layers to `~/.codex/agents/`, and appends or refreshes the
matching `[agents.<name>]` declarations in `~/.codex/config.toml`. The generated
declarations use `config_file = "agents/<name>.toml"` because Codex resolves
relative `config_file` paths from the declaring `config.toml`.

Before mutating an existing `~/.codex/config.toml`, setup writes a timestamped
`config.toml.itw-backup-*` file next to it. The setup script also compares the
packaged-resource fingerprint from the installed skill with the global `itw`
command after installation; mismatch fails the setup.

`~/.codex/agents/*.toml` and `~/.codex/config.toml` are live runtime state.
Do not treat private dotfiles as the source of truth for these shared workflow
agents.
