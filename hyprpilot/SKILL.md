---
name: hyprpilot
description: >-
  Drive and visually inspect a native GUI app (Iced, egui, GTK, winit…) on
  Hyprland through a headless output, leaving the user's desktop untouched
  (focus preserved, cursor restored) — or on an agent desktop of its own, a
  nested compositor with its own seat and cursor (`--isolated`). Use when asked
  to "test the GUI", "piloter l'app native", "screenshot the app", "voir le
  rendu", to verify a native app's rendering after a change, to interact (keys,
  text, clicks) with a native window that exposes no AT-SPI tree, or to run a
  GUI without disturbing the user's session. Do not use for web or Electron
  apps (use agent-browser), apps exposing a usable AT-SPI accessibility tree
  (prefer a semantic tool), or compositors other than Hyprland.
---

# hyprpilot

One CLI, one loop: observe → act → verify with screenshots you read yourself.
Two modes; the first decision is which one.

**shared** (default) — you drive windows on the *user's* desktop. The window is
parked on an invisible headless output; keys are delivered by window address (no
focus stolen); clicks warp a virtual pointer, then restore the cursor and
re-focus the window the user had. Pick it to act on a window the user already
has open, or when the app needs the user's real portals (file pickers).

**isolated** (`session start --isolated`) — an agent desktop that belongs to
you: a nested Hyprland with its own seat, cursor and keyboard. Nothing there can
touch the user's desktop, so there is no focus to steal and no cursor to
restore. Pick it to exercise a native app end to end without disturbing
anything.

One agent desktop per session, any number in parallel. Only one shared session
at a time, whatever its name.

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
protocol, and runtime-dir permissions, reports the active keyboard layout, and
checks what an agent desktop needs (`Hyprland` on PATH, version against the
validated 0.56).

## Canonical loop — shared

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

## Canonical loop — agent desktop

```bash
export HYPRPILOT_SESSION=agent-1            # every command below acts on it
hyprpilot session start --isolated --app './my-app' --match-title 'My App'
hyprpilot wait --stable --timeout 10s
hyprpilot shot                              # READ the PNG
hyprpilot type "fig"                        # or: hyprpilot key Down Return
hyprpilot wait --stable
hyprpilot click 120 260                     # nothing of the user's moves
hyprpilot target --match-title 'Preferences' --wait 5s   # a second toplevel
hyprpilot session show                      # let the user watch; `hide` puts it back
hyprpilot teardown                          # destroys the whole desktop
```

`--app` is required: an agent desktop starts empty, there is nothing to attach
to. Same session name = same desktop, so `HYPRPILOT_SESSION` (or
`--session NAME`) is how parallel agents stay out of each other's way.

What differs from shared mode:

- No focus or cursor to steal, so `--focus` is accepted and does nothing.
- No parking and no teardown dispositions: `target` just focuses the match
  inside the desktop, and `--untracked` / `--on-teardown` are refused.
- `windows` and `status` report the desktop's own clients and geometry, never
  the host's.
- `teardown` destroys the desktop whole, so there is no window to restore and
  `--kill`/`--close` are refused. Still always tear down: the headless output
  and the nested compositor outlive you otherwise.
- `session resize` is refused. Tear down and start again with `--size`.
- Portal file pickers never open (see the caveat below).
- `session show` / `session hide` are the only way to let the user look at the
  desktop. Do not tell them to click the `agent-*` workspace in waybar: that
  moves their focus to an invisible headless output.

## Secondary windows — shared mode (portals, pickers, "Library"-style popups)

When the app opens another window (file chooser, portal, secondary
browser window), do NOT tear down and restart. Adopt it:

```bash
hyprpilot windows                                   # JSON list, tracked/active/focused annotated
hyprpilot target --untracked --match-title "All Files" --wait 5s
#   … drive the popup (shot / click / type) …
hyprpilot target --address 0x<main-window>          # switch back; popup stays tracked, parked
hyprpilot teardown                                  # restores/closes every adopted window
```

- `target` criteria are exact-match and combined with AND; ambiguity fails
  with the candidates as a JSON array on the last stderr line — pick by
  `--address`.
- Adopted windows default to `--on-teardown restore`; use
  `--on-teardown close` for windows that should die with the session
  (e.g. a picker you opened). Switching back to a tracked window forbids
  `--on-teardown`.
- Only the active target is visible on the output; the others are parked
  invisibly. `session resize WxH` adapts the output when a dialog is
  larger than the current size (no teardown needed; the start warning
  suggests it).

In an agent desktop, `target` is the whole mechanism: it focuses the match
inside the desktop and nothing is parked or restored.

## When to use `--focus`

Shared mode only; in an agent desktop the flag is accepted and does nothing.

Default: stay focusless (`key`/`type`/`click` as-is) — it works on
winit/Iced/egui and most GTK apps. Reach for `--focus` when a widget
visibly ignores focusless input: portal file choosers, XUL/Firefox menus,
Chromium chrome shortcuts (`Ctrl+L`…). It focuses the session window for
one action, then restores the user's focus and cursor strictly (non-zero
exit if restoration fails). The physical keyboard reaches the target
during that window — keep `--focus` actions short.

- `session start` and `target` match windows by **exact** title/class
  (`hyprctl clients` values), and by exact `--pid` or `--address`. Iced/winit
  windows often have an empty class — match by title.
- Every command takes `--session NAME` (default `$HYPRPILOT_SESSION`, else
  `default`, alphabet `[a-z0-9-]{1,32}`); the whole loop above is one session.
  Reserved namespace, do not touch by hand: outputs `hyprpilot` and
  `hyprpilot-<session>`, workspaces `hyprpilot`, `special:hyprpilot-parked`
  and `agent-<session>`.
- `--size WxH` sets the output resolution: default 1600x1000 shared,
  1920x1080 for an agent desktop.
- `shot` is framed to the window (fewer tokens, no waybar); `--full` captures
  the whole output (in isolated mode, the whole agent desktop). Files land in
  `$XDG_RUNTIME_DIR/hyprpilot/sessions/<session>/shots/`.
- `wait --stable` for "the UI settled"; `wait --changed-from PNG` for "the UI
  reacted"; both take `--timeout 5s` and exit non-zero on timeout. Every
  capture underneath runs under its own bounded deadline, so a stuck grim ends
  as an error instead of hanging the command.

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

- In shared mode a click or a scroll must move the real cursor for a moment:
  position and focus are restored immediately after, but a brief flash on the
  user's screen is possible. Prefer keyboard navigation when both work. In an
  agent desktop the cursor moved is the agent's own.
- `click --double` sends two press/release pairs ~80 ms apart (one warp, one
  restore). `scroll X Y --dy N [--dx N]` counts wheel detents, positive =
  down/right; `--dy`/`--dx` both zero is an error.
- Captures are pixels, not semantics: there is no element tree. Derive click
  coordinates from a fresh `shot` plus the window geometry in `status`.
- Routing: web/Electron **content** → agent-browser; browser **chrome**,
  portals and native windows → hyprpilot. A browser's page DOM is better
  driven semantically; its file-picker dialog is not.
- Do not run concurrent hyprpilot commands on one session (state file is
  not a lock). Different sessions in parallel are fine.
- One shared session at a time, whatever its name; agent desktops are
  unlimited. If `session start` reports an existing session, run
  `hyprpilot teardown` first. A start that failed midway leaves its state on
  purpose — `teardown` cleans it up.
- File pickers work in shared mode because it drives the user's real desktop.
  An **agent desktop** cannot open them: the nested compositor inherits the
  host D-Bus session, so `FileChooser` portal calls hang with no dialog — every
  GTK4 picker included. Measured in
  `crates/hyprpilot/references/portal-probe.md`. A GUI you have to test through
  a file picker belongs in shared mode.
- If an agent desktop's nested compositor dies, every command fails saying so
  and naming `teardown`; `teardown` is what cleans it up.
- State left by an older build (`schema_version: 2`, or the unversioned
  format) is refused by every command with the version it holds and the way
  out; `hyprpilot teardown` still clears it.
- Shared `teardown` walks every tracked window in reverse adoption order:
  **attached** windows go back to their origin workspace, position and
  size (`restore`), `close`-disposition windows are actually closed, a
  **spawned** primary is closed (`--kill` kills its process group,
  `--close` closes an attached primary instead). Then the output is
  removed, and the user's cursor is put back where it was. Leaving outputs
  behind pollutes the user's monitor layout — always tear down. A corrupt
  state aborts without removing anything and tells you how to recover with
  `hyprpilot windows` + `hyprctl`.
