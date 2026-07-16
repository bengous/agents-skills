---
name: hyprpilot
description: >-
  Drive and visually inspect a native GUI app (Iced, egui, GTK, winit…) on
  Hyprland through a headless output, leaving the user's desktop untouched
  (focus preserved, cursor restored). Use when asked to "test the GUI", "piloter l'app native",
  "screenshot the app", "voir le rendu", to verify a native app's rendering
  after a change, or to interact (keys, text, clicks) with a native window
  that exposes no AT-SPI tree. Do not use for web or Electron apps (use
  agent-browser), apps exposing a usable AT-SPI accessibility tree (prefer a
  semantic tool), or compositors other than Hyprland.
---

# hyprpilot

One CLI, one loop: park the app's window on an invisible headless output, then
observe → act → verify with screenshots the agent reads. Keys are delivered by
window address (no focus stolen); clicks warp a virtual pointer, then restore
the cursor and re-focus the window the user had (if any was focused).

## Preflight

```bash
command -v hyprpilot && hyprpilot doctor
```

If the binary is missing or `doctor` fails: **STOP**. Never install silently —
show the user the install commands and ask:

```bash
# in the agents-skills checkout
cargo build --release -p hyprpilot
install -m755 target/release/hyprpilot ~/.local/bin/
```

`doctor` verifies hyprctl, the Hyprland session, grim, the virtual-pointer
protocol, and runtime-dir permissions, and reports the active keyboard layout.

## Canonical loop

```bash
hyprpilot session start --app './my-app' --match-title 'My App'   # or attach to a running window
hyprpilot wait --stable --timeout 10s   # a freshly launched app needs a moment to paint
hyprpilot status                        # JSON — compare user_active_window to initial_user_focus
hyprpilot type "fig"                    # or: hyprpilot key Down Down Return
hyprpilot wait --stable                 # replaces sleeps: polls until 2 identical frames
hyprpilot shot                          # prints the PNG path — READ the PNG to verify
hyprpilot click 120 260                 # window-relative; cursor + focus restored
hyprpilot wait --changed-from <last.png>
hyprpilot shot after-click
hyprpilot scroll 800 250 --dy 5         # wheel detents at that point; positive = down/right
hyprpilot teardown                      # ALWAYS, at the end of every session
```

Every visual assertion = read the PNG with the Read tool. Never claim a UI
state you have not seen in a capture.

- `session start` matches windows by **exact** title/class (`hyprctl clients`
  values). Iced/winit windows often have an empty class — match by title.
- `--size WxH` (default 1600x1000) sets the headless output resolution.
- `shot` is framed to the window (fewer tokens, no waybar); `--full` captures
  the whole output. Files land in `$XDG_RUNTIME_DIR/hyprpilot/shots/`.
- `wait --stable` for "the UI settled"; `wait --changed-from PNG` for "the UI
  reacted"; both take `--timeout 5s` and exit non-zero on timeout (the
  timeout bounds the polling loop, not a single hung capture).

## Keys

`key` takes XKB keysym names and chords: `a`, `Down`, `Escape`, `Return`,
`F5`, `Ctrl+c`, `Ctrl+Shift+Escape`. `type` maps printable ASCII through US
shift pairs (`!` → `SHIFT+1`). Both are delivered by window address —
validated to work without focus on winit/Iced.

Caveats:
- Accented characters (`é`, `ç`…) resolve only if the active keymap exposes
  them (e.g. layout `fr`); on a `us` keymap they fail with a clear error.
- `sendshortcut` can only reach keysyms that exist unmodified on the active
  keymap; `hyprpilot key` errors mention this when it happens.

## Caveats

- A click or a scroll must move the real cursor for a moment: position and
  focus are restored immediately after, but a brief flash on the user's
  screen is possible. Prefer keyboard navigation when both work.
- `click --double` sends two press/release pairs ~80 ms apart (one warp, one
  restore). `scroll X Y --dy N [--dx N]` counts wheel detents, positive =
  down/right; `--dy`/`--dx` both zero is an error.
- Captures are pixels, not semantics: there is no element tree. Derive click
  coordinates from a fresh `shot` plus the window geometry in `status`.
- One session at a time. If `session start` reports an existing session, run
  `hyprpilot teardown` first. A start that failed midway leaves its state on
  purpose — `teardown` cleans it up.
- `teardown` closes a **spawned** app (`--kill` kills its process group) but
  returns an **attached** window to its original workspace instead of closing
  an app the user had opened (`--close` overrides). It then removes the
  headless output. Leaving outputs behind pollutes the user's monitor
  layout — always tear down.
