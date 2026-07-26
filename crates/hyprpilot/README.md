# hyprpilot

CLI to drive and visually inspect native GUI apps (Iced, egui, GTK, winit…)
on Hyprland without touching the user's desktop. Companion binary of the
`hyprpilot` skill (`../../hyprpilot/SKILL.md`).

The app's window is parked on a dedicated headless output; keys go through
the `sendshortcut` dispatcher (by window address, no focus), clicks through a
native `zwlr_virtual_pointer_v1` with cursor and focus restored, captures
through grim framed to the window, and `wait` replaces sleeps with a native
PNG pixel diff.

## Install

```bash
cargo build --release -p hyprpilot
install -m755 target/release/hyprpilot ~/.local/bin/
```

## Sessions and modes

Every command takes a global `--session NAME` (default: `$HYPRPILOT_SESSION`,
else `default`, alphabet `[a-z0-9-]{1,32}`).

- **shared** — the mode documented in this file: the user's own windows are
  parked on the output `hyprpilot`. That output is a singleton, so a second
  shared session is refused whatever its name.
- **isolated** (`session start --isolated`) — an agent desktop: a nested
  Hyprland per session, its console window living on the active workspace of a
  per-session headless output. Under construction; every command in isolated
  mode currently fails naming the slice that implements it
  (`hyprpilot-isolated-slice-plan.md`), without touching the compositor.
  `session resize` will stay unsupported there for this cycle.

## Commands

| Command | Role |
|---|---|
| `session start [--isolated] --app CMD --match-title T [--match-class C] [--size WxH]` | launch or attach, create the headless output, park the window |
| `session resize WxH` | resize the headless output in place, re-place the active window (no teardown needed) |
| `windows` | JSON array of every Hyprland client, annotated `tracked`/`active`/`focused` — discovery without `hyprctl`+`jq` |
| `target [--address A] [--match-title T] [--match-class C] [--pid P] [--untracked] [--wait 10s] [--on-teardown restore\|close]` | adopt another window into the session (or switch back to a tracked one); the previous target is parked, invisible |
| `key <CHORDS…> [--focus]` | send key chords (`Down`, `Ctrl+c`) without focus |
| `type "text" [--focus]` | type character by character (US shift pairs, common French accents) |
| `click X Y [--button b] [--double] [--absolute] [--focus]` | virtual-pointer click (`--double`: two clicks 80 ms apart); cursor + focus restored |
| `scroll X Y --dy N [--dx N] [--absolute] [--focus]` | wheel detents at that point (positive = down/right); cursor + focus restored |
| `shot [NAME] [--full] [--out DIR]` | window-framed PNG (prints the absolute path) |
| `wait [--stable\|--changed-from PNG] [--timeout 5s]` | poll captures until stable / changed |
| `status` | session JSON: schema, tracked windows + dispositions, active target, parked windows, configured vs effective output size |
| `doctor` | environment checks (hyprctl, grim, protocols, layout) |
| `teardown [--kill] [--close]` | apply each tracked window's disposition in reverse adoption order (restore workspace + exact geometry, or close), then remove the output and state |

State lives in `$XDG_RUNTIME_DIR/hyprpilot/sessions/<name>/session.json`
(`schema_version: 3`, one claim per name, multi-window, written atomically).
Requires Hyprland (tested on 0.55), grim and zenity+jq for the E2E gate.

**Reserved namespace**: outputs `hyprpilot` (shared) and `hyprpilot-<session>`
(isolated); workspaces `hyprpilot` and `special:hyprpilot-parked` (shared) and
`agent-<session>` (isolated). Named `agent-*` workspaces show up in
`hyprctl workspaces -j` as soon as they exist, so waybar may display them
depending on its `all-outputs` setting; they are a presence indicator, not a
button (clicking one moves focus to an invisible headless output).

**No state compatibility**: a `schema_version: 2` or unversioned state file
makes every command fail with the version found, the version expected and the
way out. Only `teardown` still reads the pre-v3 location
`$XDG_RUNTIME_DIR/hyprpilot/session.json`, so a session left by an older build
can be cleaned up.

## Contracts

- **`ready` means capturable**: `session start` (and `target`, and
  `session resize`) only report success after the active window has been
  placed on the output and re-read from Hyprland — contained if it fits,
  clamped top-left with a warning suggesting `session resize` if it is
  larger than the output. Windows are never resized automatically.
- **Strict restoration**: `click`/`scroll` (and every `--focus` action)
  snapshot cursor + user focus, act, restore focus first, cursor last, and
  re-verify both (±1 px). Any restoration failure — including a concurrent
  physical mouse move, or an initial "no focus" state that cannot be
  re-established — makes the command exit non-zero with both the action
  and restoration errors reported. No retry.
- **`--focus`**: temporarily focuses the session's active window for
  widgets that ignore focusless input (GTK portals, XUL menus), inside the
  same single restoration envelope. The physical keyboard can reach the
  target while it holds focus: documented risk, no extra mechanics.
- **Matching is exact and never ambiguous**: `session start` and `target`
  fail on multiple candidates, printing a human message and a JSON array
  of the candidates as the last stderr line.
- **Teardown by disposition**: adopted windows are `restore` (workspace,
  then exact position + size for floating windows) or `close` (waits for
  the window to disappear); already-gone windows are idempotent successes.
  A corrupt or unknown-version state file aborts without touching the
  output and points to `hyprpilot windows` + raw `hyprctl` commands for
  manual recovery; the file is kept so teardown stays replayable. Without
  a state file, an orphan `hyprpilot` output is only swept when no client
  sits on it (`monitor: -1` counts as occupied).

## Limits

- Concurrent hyprpilot commands on the same session are out of contract:
  the state file is not a lock.
- Parked windows live on `special:hyprpilot-parked`, pinned to the session
  output by a `workspace` rule set at parking time; the rule is inert
  after teardown and disappears at the next Hyprland config reload. Never
  toggle that special workspace; `shot`/`wait` refuse to capture while it
  is visible on the session output.
- Removing the output on teardown makes Hyprland re-center the user's
  cursor on the remaining monitor: teardown currently does not restore the
  cursor position afterwards.
- **Portals inside an agent desktop**: an app launched in a nested Hyprland
  inherits the host's `DBUS_SESSION_BUS_ADDRESS`, so any `FileChooser` portal
  call hangs with no dialog anywhere — including every GTK4 file picker, whose
  only path is the portal (`GTK_USE_PORTAL=0` changes nothing). Measured, with
  the D-Bus traces and a validated private-bus workaround that is out of scope
  for this cycle, in [`references/portal-probe.md`](references/portal-probe.md).
  Shared mode is unaffected: it drives the user's real portal dialogs.

## Design notes

- `sendshortcut` resolves key names to keysyms reachable *unmodified* on the
  active keymap and forwards the requested modifiers — hence the US table in
  `keys.rs` mapping `!` to `SHIFT+1`, and accents working only on keymaps
  that expose them.
- Unbound virtual-pointer absolute motion is mapped by Hyprland over the
  bounding box of the whole monitor layout (`CPointerManager::warpAbsolute`);
  `pointer.rs` warps in those coordinates and verifies the landing position
  via `hyprctl cursorpos` before clicking.
- When the headless output is created, its initial empty workspace is
  evacuated to a physical monitor, otherwise grim captures the wallpaper.
- The session file is claimed atomically (`create_new`) and written before
  any compositor side effect: a start that fails midway stays recoverable
  with `teardown`, which aborts rather than remove the output while the
  window is still open. `wait --timeout` bounds the polling loop; it cannot
  preempt a single hung grim invocation.
