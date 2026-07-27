# hyprpilot

CLI to drive and visually inspect native GUI apps (Iced, egui, GTK, winit…)
on Hyprland while leaving the user's desktop as it was: every action restores
the focus and the cursor it moved (see *Strict restoration*, which also says
what `session start` itself does not restore). Companion binary of the
`hyprpilot` skill (`../../hyprpilot/SKILL.md`).

In shared mode the app's window is parked on a dedicated headless output; keys
go through the `sendshortcut` dispatcher (by window address, no focus), clicks
through a native `zwlr_virtual_pointer_v1` with cursor and focus restored,
captures through grim framed to the window, and `wait` replaces sleeps with a
native PNG pixel diff. In isolated mode the same commands drive a nested
compositor the agent owns instead of the user's desktop.

## Install

```bash
cargo build --release -p hyprpilot
install -m755 target/release/hyprpilot ~/.local/bin/
```

## Sessions and modes

Every command takes a global `--session NAME` (default: `$HYPRPILOT_SESSION`,
else `default`, alphabet `[a-z0-9-]{1,32}`).

- **shared** — the user's own windows are parked on the output `hyprpilot`.
  That output is a singleton, so a second shared session is refused whatever
  its name.
- **isolated** (`session start --isolated`) — an agent desktop: a nested
  Hyprland per session, with its own seat, cursor and keyboard, its console
  window living on the active workspace of a per-session headless output. One
  desktop per session, any number in parallel. `--app` is required (a desktop
  starts empty, there is nothing to attach to) and `--size` defaults to
  1920x1080.

Every command reads the mode from its session's state and routes before the
first compositor read: an isolated command queries the clients, monitors,
cursor and focus of *its own* instance, never the host's.

## Commands

| Command | Role |
|---|---|
| `session start [--isolated] --app CMD --match-title T [--match-class C] [--size WxH]` | launch or attach, create the headless output, park the window — or build a whole agent desktop |
| `session resize WxH` | shared only: resize the headless output in place and re-place the active window (no teardown needed); it waits for the output to report exactly `WxH` and fails otherwise |
| `session show` / `session hide` | agent desktops only: bring the console window in front of the user, or send it back to `agent-<session>` |
| `windows` | JSON array of every client, annotated `tracked`/`active`/`focused` — discovery without `hyprctl`+`jq` |
| `target (--address A \| --match-title T \| --match-class C \| --pid P \| --untracked) [--wait 10s] [--on-teardown restore\|close]` | at least one selector is required; adopt another window into the session (or switch back to a tracked one); the previous target is parked, invisible. In an agent desktop it focuses the match instead, and refuses `--untracked` and `--on-teardown` |
| `key <CHORDS…> [--focus]` | send key chords (`Down`, `Ctrl+c`) without focus |
| `type "text" [--focus]` | type character by character (US shift pairs, common French accents) |
| `click X Y [--button b] [--double] [--absolute] [--focus]` | virtual-pointer click (`--double`: two clicks 80 ms apart); cursor + focus restored |
| `scroll X Y [--dy N] [--dx N] [--absolute] [--focus]` | wheel detents at that point (positive = down/right), at least one of the two non-zero; cursor + focus restored |
| `shot [NAME] [--full] [--out DIR]` | window-framed PNG (prints the absolute path) |
| `wait [--stable\|--changed-from PNG] [--timeout 5s]` | poll captures until stable / changed |
| `status` | session JSON: schema, mode, tracked windows + dispositions, active target, parked windows, configured vs effective output size. In an agent desktop: instance signature, `WAYLAND_DISPLAY`, pid, console address, `shown`, and the geometry read inside the instance |
| `doctor` | environment checks (hyprctl, grim, protocols, layout) plus the agent-desktop ones (`Hyprland` on PATH, version against the validated 0.56, sessions directory writable) |
| `teardown [--kill] [--close]` | apply each tracked window's disposition in reverse adoption order (restore workspace + exact geometry, or close), unwind the host ledger, then remove the output and state. An agent desktop is destroyed whole and refuses both flags |

State lives in `$XDG_RUNTIME_DIR/hyprpilot/sessions/<name>/session.json`
(`schema_version: 5`, one claim per name, multi-window). It is published by a
single `hard_link`, which claims the name and makes the file appear complete in
the same step, so no reader — and no `teardown` — can find a half-written
session; `session.lock` next to it serialises the commands that change it.
Captures, `wait` scratch files, the generated nested config and its log sit in
the same directory, so parallel sessions never share a file. Requires Hyprland
(shared mode tested on 0.55, agent desktops on 0.56 — `doctor` warns on any
other version), grim, and zenity+jq for the E2E gate.

**Reserved namespace**: outputs `hyprpilot` (shared) and `hyprpilot-<session>`
(isolated); workspaces `hyprpilot` and `special:hyprpilot-parked` (shared) and
`agent-<session>` (isolated).

**What an agent desktop costs your bar, while it runs.** `hyprctl output create
headless` makes Hyprland attach the *lowest free workspace id* to the new
output — `3` if you occupy 1 and 2 — and the start renames it `agent-<session>`.
A bar with persistent workspaces then shows that button under a name its
`format-icons` has no key for, so the number is replaced by whatever `default`
draws. It is not a new button appearing: it is one of yours wearing another
name. `teardown` gives the name back before the output goes, so the bar is
whole again the moment the session ends; there is no way to avoid the
confiscation in between, because a workspace rule with `default:true` is not
applied to an output created at runtime (measured on 0.56, 2026-07-27).

Clicking that workspace moves the focus to an invisible headless output, and
`session show` then refuses to move the console onto the workspace it came from:
looking at an agent desktop is `session show`/`session hide`, never a bar click.

**What a session leaves on the host, for good.** Hyprland retracts no `keyword`
while it runs ([#5691]), and reposting one stacks a second instead of replacing
it ([#2268]). Every session therefore leaves its `monitor` mode-set rule and its
`workspace` binding behind. They are inert, and they are posed at most once — a
start reads `hyprctl workspacerules` and reuses an equivalent rule rather than
adding one. `doctor` lists what is outstanding; `hyprctl reload` is the only
thing that clears it.

[#5691]: https://github.com/hyprwm/Hyprland/issues/5691
[#2268]: https://github.com/hyprwm/Hyprland/issues/2268

**No state compatibility**: a state file from any older schema makes every
command fail with the version found, the version expected and the way out.
`teardown` is the exception, in both locations — the pre-v3
`$XDG_RUNTIME_DIR/hyprpilot/session.json`, and a `schema_version: 3` or `4` file
at the current path — so a session left by an older build is never stranded with
its window parked on a hidden output, or its agent desktop still alive. v2 and
v3 recorded no window identity, so their teardown disposes of windows by address
alone, as the build that wrote them did; v4 did record it, and keeps it.

## Agent desktops (`--isolated`)

A nested Hyprland cannot run headless (no DRM master), and a nested compositor
whose window the host stops compositing stops receiving frame callbacks: its
rendering freezes and every screencopy capture blocks for ever. Hence the whole
shape of this mode — the nested compositor's console window lives on the
**active** workspace of a headless output the host keeps compositing, invisible
to the user.

`session start --isolated --app CMD --match-title T` builds one in this order,
rewriting the state after each acquired resource so an interrupted start stays
recoverable with `teardown`:

1. claim `sessions/<name>/`; snapshot the host (workspace active on every
   output, focused window, cursor position);
2. `output create headless hyprpilot-<name>`, then mode-set `WxH@60` with
   `scale 1` imposed and read back — a headless output otherwise inherits a
   non-trivial scale;
3. rename that output's *active* workspace to `agent-<name>`
   (`moveworkspacetomonitor` would leave it inactive and freeze captures). A
   workspace holding windows, or one the snapshot saw in front of the user, is
   refused;
4. generate `sessions/<name>/hyprland.conf`: host keymap read from
   `hyprctl devices`, no animations, no gaps or borders, flat background, no
   `exec-once`;
5. spawn `Hyprland -c <conf>` behind one-shot rules `[workspace
   name:agent-<name> silent; noinitialfocus; fullscreen]`. The instance is
   identified by diffing `$XDG_RUNTIME_DIR/hypr/` and the Wayland sockets, the
   console window by `HYPRPILOT_AGENT_SESSION=<name>` in `/proc/<pid>/environ`
   plus class `aquamarine` — never by title;
6. re-read the host snapshot: any drift (an output gone, a workspace switched,
   the focus moved) fails the start and rolls back everything acquired;
7. `hyprctl -i <sig> dispatch exec CMD`, then wait for the exact match among
   the *instance's* clients and for that window to be capturable.

Then, inside the desktop:

- `key`/`type` — `sendshortcut` dispatched by the instance to the recorded
  window; `click`/`scroll` — a virtual pointer opened on the nested
  compositor's own socket, in the nested layout's coordinates, with the landing
  position verified against the instance's `cursorpos`. No cursor or focus
  envelope: that seat has no human on it, and `--focus` is accepted as a no-op.
- `shot` — grim against the nested `WAYLAND_DISPLAY`, framed on the recorded
  window; `--full` is the whole agent desktop. `wait` is unchanged on top.
- `target` — the same exact, never-ambiguous matcher, run against the
  instance's clients, then `focuswindow` inside it. No parking and no
  dispositions.
- `session show` moves the console window to the workspace the user is looking
  at, floating and pinned to the size it had (the agent desktop renders at the
  size of that window); `session hide` sends it back to `agent-<name>` at the
  configured size and the headless output's origin, then re-checks that
  `agent-<name>` is active there. Both are idempotent and leave the user's
  focus alone.
- Every command but `teardown` first checks that the recorded pid still carries
  the session marker. A crashed nested compositor therefore fails fast, naming
  `teardown`, instead of timing out or falling back to the user's desktop.

`teardown` closes the app politely inside the instance, then `dispatch exit`,
then SIGTERM, then SIGKILL, each escalation reported. Every process still
carrying `HYPRPILOT_AGENT_SESSION=<name>` is swept afterwards, which also
catches a compositor spawned but never recorded; the nested log is kept as
`sessions/<name>/instance.log` before `$XDG_RUNTIME_DIR/hypr/<sig>/` is
removed. The output goes only once the host has reaped the console window,
since removing it earlier would drop that window onto the user's desktop, and
the session directory goes last. Every step on an object already gone is an
idempotent success.

## Validation

Unit tests cover the state, the mode routing, the generated nested config, the
teardown plan and the escalation ladders; `scripts/hyprland-gate.sh all` plays
the end-to-end scenarios against a live session (opt-in, never automatic). The
gate refuses to start on a session or a reserved workspace it does not own,
holds a lock so two runs cannot share the desktop, checks between scenarios that
the desktop is back to its baseline, and reports scenarios that did not run as
`SKIP` with a non-zero exit — a gate that covered nothing is not green.
The 29 scenarios passed against Hyprland 0.56 before the window-identity and
session-lock changes; they have not been replayed since. The run
found four defects no unit test could reach: `setfloating` does not clear a
fullscreen state, so `session show` had to focus the console to drop it;
attributing a nested compositor by diffing the instance directory is unsound
under concurrent starts, and identity now comes from `hyprctl instances`; an
unparseable v3 state file lost its recovery instruction; and a console shown on
the user's workspace warped their cursor as it died. One scenario
(`isolated_show_hide`) failed once in four runs on a start whose console never
mapped — not reproduced since, and a failing start now prints the tail of the
nested compositor's log before the rollback removes it.

## Contracts

- **`ready` means capturable**: `session start` (and `target`, and
  `session resize`) only report success after the active window has been
  placed on the output and re-read from Hyprland: on the session workspace,
  that workspace active on the output, no special workspace in front. For a
  **floating** window the geometry is verified too — contained if it fits,
  clamped top-left with a warning suggesting `session resize` if it is larger
  than the output. A tiled window is laid out by Hyprland, so only its
  workspace is checked. Windows are never resized automatically.
- **Strict restoration**: `click`/`scroll` (and every `--focus` action)
  snapshot cursor + user focus, act, restore focus first, cursor last, and
  re-verify both (±1 px). Any restoration failure — including a concurrent
  physical mouse move, or an initial "no focus" state that cannot be
  re-established — makes the command exit non-zero with both the action
  and restoration errors reported. No retry. A panic inside the action
  restores the desktop before it unwinds; `SIGINT`/`SIGTERM` are **not**
  caught, so killing a command mid-action (Ctrl-C during a long `type
  --delay-ms`) can leave the focus and the cursor where the action put them
  — `hyprpilot teardown` puts the desktop back.
- **Which window a command drives**: a tracked window is recorded as its
  address *and* Hyprland's `stableId`, because the address is a pointer the
  compositor reuses. Every action, park, restore and close re-reads both
  first; if the address now belongs to a window this session never adopted,
  nothing is sent to it (actions fail naming it, teardown leaves it alone).
- **One mutating command at a time**: `session start`, `target`,
  `session resize`, `session show`/`hide` and `teardown` hold the session lock
  for their whole read → change → persist → drive sequence, so two of them can
  no longer overwrite each other's window table. `start` also holds a
  machine-wide `shared.lock` from the singleton check to the published claim,
  so two starts under different names cannot both take the one shared output.
  The second command exits non-zero without touching anything. Read-only
  commands (`shot`, `wait`, `status`, `windows`) and the input commands never
  wait for it.
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
  `--kill` targets the spawned process group, not its window, so it still
  runs when the app closed its window without exiting; a group already dead is
  a success, and the window's own disposition still applies, so an app that
  re-parented itself does not leave its window on the output. The group is
  identified by its pid *and* the start time recorded at spawn: a pid the
  kernel has since handed to someone else is never signalled. A corrupt or unknown-version state file aborts
  without touching the output and points to `hyprpilot windows` + raw
  `hyprctl` commands for manual recovery; the file is kept so teardown stays
  replayable.
- **The output only goes when it is empty**: teardown removes the session
  output it created (one it found already there is left alone) only after
  re-reading the compositor and finding no client on it — briefly polled, so
  a window still unmapping is waited for rather than treated as occupancy. A
  window that landed there by another route stops the removal, and the state
  is kept so the teardown can be replayed once it is moved off. Without a
  state file the orphan sweep runs the same check, but stricter: with nothing
  to attribute a client with, one the compositor places nowhere
  (`monitor: -1`) counts as occupied.

## Limits

- Concurrency *between* sessions is fine — separate state directories,
  separate instances. On one session, the mutating commands take the session
  lock (above); the input and capture commands do not, so a `click` fired
  while a `target` is still moving windows can act on the window that is on
  its way out.
- Parked windows live on `special:hyprpilot-parked`, pinned to the session
  output by a `workspace` rule set at parking time; the rule is inert
  after teardown and disappears at the next Hyprland config reload. Never
  toggle that special workspace; `shot`/`wait` refuse to capture while it
  is visible on the session output.
- `session resize` is unsupported in isolated mode: an agent desktop keeps the
  size it was started with, so resizing means `teardown` then
  `session start --isolated --size WxH`.
- **Portals inside an agent desktop**: an app launched in a nested Hyprland
  inherits the host's `DBUS_SESSION_BUS_ADDRESS`, so any `FileChooser` portal
  call hangs with no dialog anywhere — including every GTK4 file picker, whose
  only path is the portal (`GTK_USE_PORTAL=0` changes nothing). Giving each
  instance its own D-Bus session bus lifts this, measured on 0.56 and out of
  scope for this cycle: the session lifecycle owns no bus.
  Shared mode is unaffected: it drives the user's real portal dialogs.
- Out of scope for this cycle: audio inside an agent desktop, and a waybar
  module for agent desktops.

Two limits of the shared mode are now lifted, in both modes:

- **The cursor comes back.** `hyprctl output remove` re-centres the user's
  cursor, so every output removal — either mode's teardown, the orphan sweep, a
  rolled-back isolated start — reads `cursorpos` immediately before and warps
  back to it afterwards, verified.
- **No capture can hang.** grim blocks for ever on a compositor that stopped
  answering screencopy, so every capture runs under a bounded deadline (5 s,
  then SIGTERM, then SIGKILL one second later). In an agent desktop a timeout
  names the one documented cause, the agent workspace no longer being active on
  its headless output, and the `grim -o hyprpilot-<session>` host-side
  fallback.

## Design notes

- `sendshortcut` resolves key names to keysyms reachable *unmodified* on the
  active keymap and forwards the requested modifiers — hence the US table in
  `keys.rs` mapping `!` to `SHIFT+1`, and accents working only on keymaps
  that expose them.
- Unbound virtual-pointer absolute motion is mapped by Hyprland over the
  bounding box of the whole monitor layout (`CPointerManager::warpAbsolute`);
  `pointer.rs` warps in those coordinates and verifies the landing position
  via `hyprctl cursorpos` before clicking.
- When the shared headless output is created, its initial empty workspace is
  evacuated to a physical monitor, otherwise grim captures the wallpaper. An
  agent desktop needs the opposite: that workspace is renamed in place so it
  stays the active one.
- The session file is claimed and written in one step (`hard_link` of a
  fully-written sibling, which fails if the name is taken), before any
  compositor side effect: a start that fails after that point stays
  recoverable with `teardown`, which aborts rather than remove the output
  while the window is still open. `--app` runs *before* the claim — the app
  has to map a window for the session to describe it — so a start that fails
  before the claim kills the process group it spawned instead of leaving it
  behind, whatever made it fail.
- Which compositor a command talks to is a value, `hypr::Ctl::{Host,
  Instance(signature)}`, resolved from the session's mode and turned into an
  `hyprctl -i <signature>` prefix: an isolated command has no path to the
  user's compositor, and the routing is unit-tested without one.
