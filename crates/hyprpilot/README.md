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
| `key <CHORDS…>` | send key chords (`Down`, `Ctrl+c`) without focus |
| `type "text"` | type character by character (US shift pairs, common French accents) |
| `click X Y [--button b] [--double] [--absolute]` | virtual-pointer click (`--double`: two clicks 80 ms apart); cursor + focus restored |
| `scroll X Y --dy N [--dx N] [--absolute]` | wheel detents at that point (positive = down/right); cursor + focus restored |
| `shot [NAME] [--full] [--out DIR]` | window-framed PNG (prints the absolute path) |
| `wait [--stable\|--changed-from PNG] [--timeout 5s]` | poll captures until stable / changed |
| `status` | session JSON: window, output, user focus |
| `doctor` | environment checks (hyprctl, grim, protocols, layout) |
| `teardown [--kill] [--close]` | close/kill a spawned app (attached windows go back to their origin workspace), remove the output, clear state |

State lives in `$XDG_RUNTIME_DIR/hyprpilot/session.json` (one session at a
time). Requires Hyprland (tested on 0.55) and grim.

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
