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

## Commands

| Command | Role |
|---|---|
| `session start --app CMD --match-title T [--match-class C] [--size WxH]` | launch or attach, create the headless output, park the window |
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

State lives in `$XDG_RUNTIME_DIR/hyprpilot/session.json` (one session at a
time, `schema_version: 2`, multi-window, written atomically; the legacy
unversioned format is only readable by `teardown`). Requires Hyprland
(tested on 0.55), grim and zenity+jq for the E2E gate.

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
- The output name `hyprpilot` is a reserved namespace.
- Removing the output on teardown makes Hyprland re-center the user's
  cursor on the remaining monitor: teardown currently does not restore the
  cursor position afterwards.

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
