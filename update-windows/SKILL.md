---
name: update-windows
description: >-
  Update a Windows machine end to end: inventory what is pending, upgrade every
  installed app with winget under a single UAC elevation instead of one prompt
  per installer, then install Windows Update patches, drivers, firmware, and
  Defender definitions, tracking progress in a log and reporting whether a
  reboot is required. Use when asked to "update my PC", "mets à jour mon PC",
  "update my apps", "mets à jour mes logiciels", "fais les mises à jour",
  "check for driver updates", "are my apps up to date", or when only one half
  is named (apps only, or Windows Update only). Skip for a repository's
  dependencies, lockfiles, or toolchain (that is batch-update), for installing
  or removing one named app, and on hosts that are not Windows.
compatibility: >-
  Windows 10/11 with winget (App Installer) and PowerShell 5.1 or later.
  Harness-neutral: needs only a shell that can run powershell.exe and read a
  file. No background-task or file-watching tool required.
---

# update-windows

Four phases: inventory, apps, Windows Update, report. Two invariants shape every
step. A machine-wide installer run from a non-elevated session raises its own UAC
prompt, so twenty pending apps means twenty prompts; elevating one parent process
once makes every installer inherit the token and costs one prompt. And nothing is
rebooted, uninstalled, or silently skipped without the user saying so first.

## Gate

Stop unless the host is Windows and `winget --version` answers. On a non-Windows
host, or when winget is absent (Server SKUs, stripped images), say so and stop
rather than falling back to a package manager the user did not ask for.

## Running the snippets

Every snippet below is a PowerShell command containing **no single quote**, so any
shell can pass it verbatim inside single quotes:

```
powershell -NoProfile -Command '<snippet>'
```

From PowerShell itself, drop the wrapper and paste the snippet. `<skill_dir>` is
this skill's directory, `<log>` an absolute path in a temp directory, one file per
phase. Never build a nested `Start-Process -Verb RunAs` invocation around the
scripts: they elevate themselves, so a path containing a space cannot break the
quoting.

## Contracts

- Ask for scope before touching anything when the request does not settle it:
  everything, apps only, or Windows Update only.
- Announce every UAC prompt before it appears, naming the window the user will see.
- **Announce any uninstall before running it.** Watching an uninstall scroll past
  unannounced is alarming even when the reinstall is immediate.
- Never reboot. Report that a reboot is required and let the user pick the moment.
- Never pass `--include-pinned`. It defeats the pins this workflow relies on.
- Remove only the pins this run added, and check `winget pin list` first: a pin
  that predates the run is a user decision.

## Workflow

### 1. Inventory (read-only)

```
winget upgrade --include-unknown
```

```
(New-Object -ComObject Microsoft.Update.Session).CreateUpdateSearcher().Search("IsInstalled=0").Updates |
    Select-Object -ExpandProperty Title
```

Report both counts, then confirm scope. `--include-unknown` matters: apps whose
installed version winget cannot read are hidden without it, and they are often the
stalest ones.

### 2. Apps via winget

Pin the session hosts first (see below), then run the script from an ordinary
session — it elevates itself:

```
powershell -NoProfile -ExecutionPolicy Bypass -File <skill_dir>\scripts\winget-upgrade-all.ps1 <log>
```

Tell the user a **Windows PowerShell** UAC prompt is about to appear and must be
accepted, and that a burst of Microsoft Defender "submit files to Microsoft"
notifications is expected — many fresh installers at once — and safe to dismiss.

Poll the log (below). When it finishes, read the failures before reporting.

### 3. Windows Update

```
powershell -NoProfile -ExecutionPolicy Bypass -File <skill_dir>\scripts\windows-update.ps1 <log>
```

Installs patches, drivers, firmware, and Defender definitions through
PSWindowsUpdate, installing that module first if missing, and never reboots.
Announce this second UAC prompt too. If phase 2 left work needing elevation (a
blocked package to uninstall and reinstall), fold it into this same elevated pass
so the user sees one prompt, not two.

`MODULE_UNAVAILABLE` in the log means PSGallery was unreachable, not that Windows
Update failed. Fall back to Settings → Windows Update → Check for updates and say
which path you took.

### 4. Report, then the host packages

Deliver the report **before** touching the pinned session hosts, so the user has
the full summary even if updating the host kills the session. Then, per pinned id:

```
winget pin remove --id <Id>
```

```
Start-Process winget -WindowStyle Hidden -ArgumentList "upgrade","--id","<Id>","--silent","--accept-package-agreements","--accept-source-agreements"
```

Detached, so the upgrade survives the session it replaces.

## Session host packages

Anything hosting this session — agent app, terminal, IDE, language runtime — dies
mid-run if winget replaces it. Resolve the ancestor chain instead of hard-coding
names:

```
$id = $PID
while ($id) {
    $p = Get-CimInstance Win32_Process -Filter "ProcessId=$id"
    if (-not $p) { break }
    "{0}`t{1}" -f $p.Name, $p.ExecutablePath
    $id = $p.ParentProcessId
}
```

Match each ancestor against `winget upgrade --include-unknown` by executable name,
publisher directory, or install location, and pin what matches:

```
winget pin add --id <Id>
```

Two exceptions upgrade in place and need no pin: `Git.Git` and
`Microsoft.PowerShell` replace files rather than the running process.

## Log polling

Use the harness's own file-watching or background-task mechanism if it has one.
Otherwise run this bounded wait and call it again while it keeps timing out:

```
$log = "<log>"
for ($i = 0; $i -lt 90; $i++) {
    if ((Test-Path $log) -and (Select-String -Path $log -Pattern "^=== Done" -Quiet)) { break }
    Start-Sleep -Seconds 1
}
Get-Content -Path $log -Tail 40
```

Both scripts write `=== Start <iso> ===` first and always terminate the log with
`=== Done <iso> exit=<code> ===`, including on crash and on refused elevation, so
this loop cannot wait forever. Progress signals are locale-independent by design —
match on shape, never on words, which differ per system language:

| Signal | Pattern |
| --- | --- |
| Position in the batch | `\(\d+/\d+\)` |
| Package being handled | `\[[\w.+-]+\]` |
| A failure | `0x[0-9A-Fa-f]{8}` |
| Refused UAC prompt | `ELEVATION_REFUSED`, then `exit=1223` |
| PSGallery unreachable | `MODULE_UNAVAILABLE` |
| Reboot verdict | `RebootRequired:` |

## Known traps

- **Git breaks the Bash tool while it updates.** `bash.exe` disappears during
  reinstall. Shell calls failing with "bash.exe not found" are transient: wait
  about a minute and retry. Prefer PowerShell for the rest of the run.
- **Different install technology.** When a publisher switches installer format,
  winget refuses the upgrade and asks for a manual uninstall and reinstall. The fix
  is `winget uninstall --id <Id>` then `winget install --id <Id>`; user settings
  survive. Announce it first — see Contracts.
- **False leftovers.** After the batch, some packages still appear pending.
  Microsoft Teams reports its old version until restarted, and PowerShell can leave
  a phantom entry. Verify before calling it a failure:
  `pwsh -Command '$PSVersionTable.PSVersion'`.
- **Hidden windows never prompt twice.** Anything interactive inside the elevated
  hidden window hangs forever, which is why the scripts pass
  `--disable-interactivity` and `-Confirm:$false`. Keep it that way.
- **A non-elevated batch means one UAC prompt per machine-wide installer.** If the
  scripts are bypassed and winget is run directly from the session, expect the
  prompt storm this skill exists to avoid.

## Final report

State, in this order: apps upgraded, apps failed with the reason per app, packages
left pinned or skipped and why, Windows updates installed, and whether a reboot is
required. Explain every line still listed by a closing
`winget upgrade --include-unknown` — false leftover, real failure, or pin. An
unexplained line reads as a silent failure.
