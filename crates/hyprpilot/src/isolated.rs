//! `session start --isolated`: builds an agent desktop, a nested Hyprland whose
//! console window lives on the **active** workspace of a host headless output.
//! The host keeps compositing that output, so the nested compositor keeps
//! receiving frame callbacks and captures never block (facts §2.2 and §2.3 of
//! `docs/superpowers/specs/2026-07-24-hyprpilot-isolated-design.md`).
//!
//! Nothing here acts on a window the user owns: the only host-side window this
//! module touches is the console it spawned itself, identified by address plus
//! the environment marker below.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::capture;
use crate::error::{Error, RestoreFailure};
use crate::guard;
use crate::host;
use crate::host::ledger::{self, HostMutation, Unwound, unwind};
use crate::hypr::{self, Ctl};
use crate::session::{self, Instance, Isolated, Ledger};

/// Injected into the nested compositor's environment at spawn and refused at the
/// top of every command: an output created *inside* a nested Hyprland stays 0x0
/// (fact §2.7), so this machinery must only ever run on the user's session.
/// Every process of an agent desktop inherits it, which is how it names a
/// desktop — but only a desktop: a shell that exported it, and anything launched
/// from inside one, carries it too, so it never identifies a process this tool
/// owns.
pub const AGENT_SESSION_ENV: &str = "HYPRPILOT_AGENT_SESSION";
/// Injected next to it at spawn, carrying a nonce only that one start knows, and
/// persisted in its state. A process is this desktop's only when it carries
/// **both**: an inherited or hand-exported session marker then selects nothing,
/// which is what keeps a sweep from signalling the caller's own shell.
pub const AGENT_INSTANCE_ENV: &str = "HYPRPILOT_AGENT_INSTANCE";
/// Class of the window an aquamarine-backed nested Hyprland maps on the host
/// (fact §2.5). Its pid is the nested compositor's pid.
const CONSOLE_CLASS: &str = "aquamarine";
/// The console title carries the *nested* compositor's own output name, not its
/// Wayland socket, so it only ever confirms the window kind (fact §2.5).
const CONSOLE_TITLE_PREFIX: &str = "aquamarine - WAYLAND-";
/// An agent desktop is a nested Hyprland (fact §2.1), so `doctor` names this
/// binary too.
pub const NESTED_BINARY: &str = "Hyprland";
const NESTED_CONFIG_FILE: &str = "hyprland.conf";
const NESTED_LOG_FILE: &str = "hyprland.log";
/// Where teardown keeps the nested compositor's own log, next to the session's
/// (the redirected stdout of the spawn), before the instance directory goes.
const INSTANCE_LOG_FILE: &str = "instance.log";
const INSTANCE_APPEAR_TIMEOUT: Duration = Duration::from_secs(15);
const READY_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a rolled-back agent desktop gets to exit on `dispatch exit` and
/// `SIGTERM` before `SIGKILL`.
const EXIT_GRACE: Duration = Duration::from_secs(2);
/// §6.2, on the registered pid and on the marker sweep alike: `dispatch exit`
/// first, then `SIGTERM`, then `SIGKILL`, then the teardown refuses to remove the
/// output.
const EXIT_ESCALATION: session::Escalation = session::Escalation {
    polite: EXIT_GRACE,
    term: EXIT_GRACE,
    kill: Duration::from_secs(1),
    poll: session::POLL_INTERVAL,
};
/// §6.1 is a courtesy, not a wait: an app that wants longer than this to unmap
/// is taken down by `dispatch exit`.
const POLITE_CLOSE_TIMEOUT: Duration = Duration::from_secs(1);

/// Session-independent half of the generated nested config (§4.4).
const LAYOUT_BLOCKS: &str = "general {
    gaps_in = 0
    gaps_out = 0
    border_size = 0
}

decoration {
    rounding = 0
    blur {
        enabled = false
    }
    shadow {
        enabled = false
    }
}

animations {
    enabled = false
}

misc {
    disable_hyprland_logo = true
    disable_splash_rendering = true
    force_default_wallpaper = 0
    background_color = rgb(1e1e2e)
    disable_autoreload = true
    # Every on-screen notice lands in `--full` captures, and all three of these
    # are host concerns: the watchdog banner (started without start-hyprland),
    # the XDG environment check and the guiutils check.
    disable_watchdog_warning = true
    disable_xdg_env_checks = true
    disable_hyprland_guiutils_check = true
}
";

const DEBUG_BLOCK: &str = "debug {
    # The nested log is the only diagnostic a failed start leaves behind, and
    # Hyprland disables it by default.
    disable_logs = false
}
";

/// One bounded observation: the value once the state is reached, or a
/// description of what was seen instead.
type Probe<T> = Result<Result<T, String>, Error>;

/// The immutable part of a start: what every state write repeats, plus what the
/// app match needs.
struct Start<'a> {
    name: &'a str,
    dir: PathBuf,
    path: PathBuf,
    output: String,
    workspace: String,
    /// Nonce injected at spawn and persisted, so every later command and every
    /// sweep identifies this desktop's processes by both markers.
    nonce: String,
    size: [u32; 2],
    command: &'a str,
    criteria: session::Criteria<'a>,
}

/// A discovered nested compositor.
#[derive(Clone)]
struct Live {
    signature: String,
    wayland_display: String,
    pid: u32,
    console_address: String,
}

impl Live {
    fn instance(&self) -> Instance {
        Instance::Live {
            signature: self.signature.clone(),
            wayland_display: self.wayland_display.clone(),
            pid: self.pid,
            console_address: self.console_address.clone(),
        }
    }
}

/// What the start must leave untouched (§4.6): the workspace active on every
/// host output and the user's focused window. The cursor is where
/// `output remove` has to warp back to (fact §2.8).
struct HostSnapshot {
    workspaces: BTreeMap<String, String>,
    /// Every workspace name that existed before this start created its output.
    /// One of them is the user's, whatever it holds and wherever it is: only a
    /// workspace the new output brought with it may be renamed (§12.1).
    workspace_names: BTreeSet<String>,
    active_window: Option<String>,
    cursor: (i32, i32),
}

/// The XKB configuration an agent desktop inherits from the host: read from
/// `hyprctl devices`, never guessed.
#[derive(Debug, PartialEq, Eq)]
pub struct Keymap {
    rules: String,
    model: String,
    layout: String,
    variant: String,
    options: String,
}

pub fn start(
    name: &str,
    app: Option<&str>,
    match_title: Option<&str>,
    match_class: Option<&str>,
    size: &str,
) -> Result<String, Error> {
    refuse_when_nested()?;
    // Held for the whole build: a `teardown` of this name would otherwise clear
    // the state between two of the steps below, and everything acquired after
    // that point would outlive any record of it.
    let _lock = session::lock_new_session(name)?;
    let command = app.ok_or_else(|| Error::Invalid {
        what: "session start --isolated",
        value: "(no --app)".to_owned(),
        hint: "an agent desktop starts empty; pass --app CMD to launch the app inside it"
            .to_owned(),
    })?;
    if match_title.is_none() && match_class.is_none() {
        return Err(Error::Invalid {
            what: "match criteria",
            value: "(none)".to_owned(),
            hint: "pass --match-title and/or --match-class".to_owned(),
        });
    }
    let size: [u32; 2] = session::parse_size(size)?.into();
    ensure_nested_binary()?;

    let start = Start {
        name,
        dir: session::session_dir(name)?,
        path: session::session_path(name)?,
        output: output_name(name),
        workspace: workspace_name(name),
        nonce: instance_nonce()?,
        size,
        command,
        criteria: session::Criteria {
            address: None,
            title: match_title,
            class: match_class,
            pid: None,
        },
    };

    session::claim_preflight(name, &start.path)?;
    ensure_output_absent(&hypr::monitors()?, &start.output, name)?;
    // Read before the first mutation and re-read at §4.6.
    let host = host_snapshot()?;

    // The state is the only record of what this start acquires, so it is also
    // the only thing the rollback and a later `teardown` can work from: what
    // used to be an in-memory `Acquired` beside it could disagree with it, and
    // never survived a kill.
    let mut ledger = Ledger::new(&start.path, name, start.initial_payload());
    ledger.claim()?;

    match start.build(&mut ledger, &host) {
        Ok(message) => Ok(message),
        Err(error) => Err(start.rolled_back(error, &ledger.payload, &host)),
    }
}

type Build<'a> = Ledger<'a, Isolated>;

impl Start<'_> {
    /// Steps 2 to 7 of §4, each persisting what it acquired before moving on.
    fn build(&self, state: &mut Build<'_>, host: &HostSnapshot) -> Result<String, Error> {
        self.create_output(state)?;
        self.rename_workspace(state, host)?;
        let config = self.write_config()?;

        let live = self.spawn_instance(state, &config)?;
        state.payload.instance = live.instance();
        state.record()?;

        check_host(host)?;

        let window = self.launch_app(&live.signature)?;
        state.payload.active_address = Some(window.address.clone());
        state.record()?;
        self.wait_until_ready(&live, &window.address)?;
        // §4.7: `ready` = the window is capturable, so the start proves it with a
        // real capture through the socket it recorded instead of inferring it
        // from what the compositors answer. A socket handed to the wrong instance
        // — the risk of a concurrent start — fails exactly here.
        capture::probe(self.name)?;
        self.warn_on_nested_size(&live);

        Ok(format!(
            "agent desktop `{}` ready — window {} (`{}`) in nested instance {} ({}, pid {}) on \
             workspace {} of output {} ({}x{})",
            self.name,
            window.address,
            window.title,
            live.signature,
            live.wayland_display,
            live.pid,
            self.workspace,
            self.output,
            self.size[0],
            self.size[1],
        ))
    }

    /// The state as the claim publishes it: every resource named, none acquired.
    fn initial_payload(&self) -> Isolated {
        Isolated {
            output: self.output.clone(),
            workspace: self.workspace.clone(),
            instance_nonce: self.nonce.clone(),
            size: self.size,
            shown: false,
            active_address: None,
            instance: Instance::Pending,
            host: Vec::new(),
        }
    }

    /// §4.2. Resolution *and* scale are imposed: a headless output otherwise
    /// inherits a non-trivial scale (fact §2.10).
    fn create_output(&self, state: &mut Build<'_>) -> Result<(), Error> {
        // The ledger entry is on disk before the output exists, whether or not
        // the mode-set below applies and whether or not the check ever passes:
        // the rollback has to remove it either way, or it stays in the user's
        // layout with no session state left to find it (§4.1).
        state.apply(
            HostMutation::OutputCreated {
                output: self.output.clone(),
            },
            || host::output_create_headless(&self.output),
        )?;
        state.apply(
            HostMutation::MonitorRuleSet {
                rule: hypr::headless_monitor_rule(&self.output, self.size[0], self.size[1]),
            },
            || host::keyword_monitor(&self.output, self.size[0], self.size[1]),
        )?;
        let [width, height] = self.size;
        poll_until(
            session::WINDOW_PLACE_TIMEOUT,
            || {
                Ok(match self.host_output()? {
                    Some(monitor) if output_is_configured(&monitor, self.size) => Ok(()),
                    Some(monitor) => Err(format!(
                        "{}x{} at scale {}",
                        monitor.width, monitor.height, monitor.scale
                    )),
                    None => Err("absent".to_owned()),
                })
            },
            |observed| {
                format!(
                    "output {} to report {width}x{height} at scale 1 (last observed: {observed})",
                    self.output
                )
            },
        )
    }

    /// §4.3. Renaming the workspace the host made active on the headless output
    /// is what keeps it active; `moveworkspacetomonitor` would leave it
    /// inactive and freeze every capture (fact §2.3).
    fn rename_workspace(&self, state: &mut Build<'_>, host: &HostSnapshot) -> Result<(), Error> {
        let monitor = self.host_output()?.ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!(
                "output {} vanished before its workspace was named",
                self.output
            ),
        })?;
        let current = monitor.active_workspace;
        if let Err(reason) = renameable(host, &hypr::clients()?, &current.name) {
            return Err(Error::Tool {
                command: "hyprctl dispatch renameworkspace".to_owned(),
                message: format!(
                    "workspace {} is active on output {} but {reason} — refusing to rename a \
                     workspace the user owns",
                    current.name, self.output
                ),
            });
        }

        // `from` goes in the ledger before the rename happens: it is the only
        // record of the name the workspace had, and losing it is what left a
        // dead `agent-*` label in the user's bar until waybar was restarted.
        state.apply(
            HostMutation::WorkspaceRenamed {
                id: current.id,
                from: current.name.clone(),
                to: self.workspace.clone(),
            },
            || host::rename_workspace(current.id, &self.workspace),
        )?;
        poll_until(
            session::WINDOW_PLACE_TIMEOUT,
            || {
                Ok(match self.host_output()? {
                    Some(monitor) if monitor.active_workspace.name == self.workspace => Ok(()),
                    Some(monitor) => Err(monitor.active_workspace.name),
                    None => Err("output absent".to_owned()),
                })
            },
            |observed| {
                format!(
                    "workspace {} to be active on output {} (last observed: {observed})",
                    self.workspace, self.output
                )
            },
        )?;
        // Bound to its output, like the parked workspace of shared mode: if this
        // workspace is destroyed while the console is shown, `hide` recreates it
        // with `movetoworkspacesilent`, and without the rule that recreation
        // lands on the monitor the console currently sits on — the user's.
        state.apply(
            HostMutation::WorkspaceRuleSet {
                rule: host::workspace_rule(&self.workspace, &self.output),
            },
            || host::keyword_workspace(&self.workspace, &self.output),
        )
    }

    /// §4.4. The keymap is the only dynamic part, so it is read from the host,
    /// never defaulted.
    fn write_config(&self) -> Result<PathBuf, Error> {
        let keymap = host_keymap()?;
        let path = self.dir.join(NESTED_CONFIG_FILE);
        fs::write(&path, nested_config(self.name, self.size, &keymap)).map_err(|source| {
            Error::Io {
                context: format!("writing nested config {}", path.display()),
                source,
            }
        })?;
        Ok(path)
    }

    /// §4.5. The one-shot window rules keep the spawn from stealing focus
    /// (fact §2.4); discovery then identifies the instance by diffing what the
    /// spawn created.
    fn spawn_instance(&self, state: &mut Build<'_>, config: &Path) -> Result<Live, Error> {
        let log = self.dir.join(NESTED_LOG_FILE);
        let command = spawn_command(&self.marker(), &self.workspace, config, &log)?;
        let windows_before = hypr::clients()?
            .into_iter()
            .map(|client| client.address)
            .collect::<BTreeSet<_>>();

        hypr::dispatch(&["exec", &command])?;

        // Identity, never arrival order: the nested compositor carries both
        // markers of this start, so the host's instance table attributes a
        // signature *and* a socket to it whatever else is being born at the same
        // moment. Diffing the runtime directory could not — measured on the first
        // live run, two concurrent starts each saw two new entries and neither
        // could claim one, and the single-entry case attributed without ever
        // checking whose it was.
        let instance = self.wait_for_marked_instance(&log)?;
        // Persisted before the console is looked for: from here on the compositor
        // has a runtime directory to exit politely and to remove (fact §2.9), and
        // neither a failure below nor a kill of this process may throw that away.
        state.payload.instance = Instance::Spawned {
            signature: instance.instance.clone(),
        };
        state.record()?;
        let console = self.wait_for_console(&windows_before, &log)?;
        let (signature, wayland_display) = (instance.instance, instance.wl_socket);
        if !console.title.starts_with(CONSOLE_TITLE_PREFIX) {
            let _ = writeln!(
                std::io::stderr(),
                "hyprpilot: warning: console window {} has title `{}`, expected \
                 `{CONSOLE_TITLE_PREFIX}…`",
                console.address,
                console.title
            );
        }
        self.ensure_console_on_workspace(&console)?;

        Ok(Live {
            signature,
            wayland_display,
            pid: console_pid(&console)?,
            console_address: console.address,
        })
    }

    /// A compositor registers itself in the host's instance table while it starts,
    /// which is the first moment a start can name what it spawned — and it names
    /// it by ownership, not by whatever appeared meanwhile.
    fn wait_for_marked_instance(&self, log: &Path) -> Result<hypr::InstanceInfo, Error> {
        let marker = self.marker();
        poll_until(
            INSTANCE_APPEAR_TIMEOUT,
            || {
                let instances = hypr::instances()?;
                let ours = |pid: i32| process_carries_marker(pid, &marker);
                Ok(marked_instance(&instances, &ours)?.cloned().ok_or_else(|| {
                    format!("{} live instances, none of them ours", instances.len())
                }))
            },
            |observed| {
                format!(
                    "the nested Hyprland of agent desktop `{name}` ({AGENT_SESSION_ENV}={name}, \
                     {AGENT_INSTANCE_ENV}={nonce}) (last observed: {observed}); nested log: {}",
                    log.display(),
                    name = self.name,
                    nonce = self.nonce,
                )
            },
        )
    }

    fn wait_for_console(
        &self,
        before: &BTreeSet<String>,
        log: &Path,
    ) -> Result<hypr::Client, Error> {
        let marker = self.marker();
        poll_until(
            session::WINDOW_APPEAR_TIMEOUT,
            || {
                let clients = hypr::clients()?;
                let ours = |pid: i32| process_carries_marker(pid, &marker);
                let console = select_console(&clients, before, &ours)?;
                Ok(console
                    .cloned()
                    .ok_or_else(|| format!("{} host windows, none of them ours", clients.len())))
            },
            |observed| {
                format!(
                    "the console window of agent desktop `{name}` (class {CONSOLE_CLASS}, \
                     {AGENT_SESSION_ENV}={name}, {AGENT_INSTANCE_ENV}={nonce}) (last observed: \
                     {observed}); nested log: {}",
                    log.display(),
                    name = self.name,
                    nonce = self.nonce,
                )
            },
        )
    }

    /// The one-shot `workspace` rule applies when the window maps; if the
    /// compositor mapped it elsewhere, move it silently and insist (§4.5).
    fn ensure_console_on_workspace(&self, console: &hypr::Client) -> Result<(), Error> {
        if console.workspace.name == self.workspace {
            return Ok(());
        }
        hypr::dispatch(&[
            "movetoworkspacesilent",
            &format!("name:{},address:{}", self.workspace, console.address),
        ])?;
        poll_until(
            session::WINDOW_PLACE_TIMEOUT,
            || {
                Ok(match find_client(&hypr::clients()?, &console.address) {
                    Some(client) if client.workspace.name == self.workspace => Ok(()),
                    Some(client) => Err(client.workspace.name.clone()),
                    None => Err("window gone".to_owned()),
                })
            },
            |observed| {
                format!(
                    "console window {} to sit on workspace {} (last observed: {observed})",
                    console.address, self.workspace
                )
            },
        )
    }

    /// §4.7. A shell spawn would die on `SIGHUP` (fact §2.6), so the nested
    /// compositor launches the app itself.
    fn launch_app(&self, signature: &str) -> Result<hypr::Client, Error> {
        hypr::dispatch_on(Ctl::Instance(signature), &["exec", self.command])?;
        poll_until(
            session::WINDOW_APPEAR_TIMEOUT,
            || {
                let clients = hypr::clients_on(Ctl::Instance(signature))?;
                match session::resolve(&clients, &self.criteria) {
                    session::Resolution::Unique(client) => Ok(Ok(client.clone())),
                    session::Resolution::None => Ok(Err(format!(
                        "{} windows in the agent desktop",
                        clients.len()
                    ))),
                    session::Resolution::Ambiguous(candidates) => {
                        Err(session::ambiguous_error(&self.criteria, candidates))
                    }
                }
            },
            |observed| {
                format!(
                    "a window matching {} in agent desktop `{}` after launching `{}` (last \
                     observed: {observed})",
                    session::criteria_label(&self.criteria),
                    self.name,
                    self.command
                )
            },
        )
    }

    /// `ready` = the window is capturable, the v2 contract. In an agent desktop
    /// that means the agent workspace is the active one on its host headless
    /// output (facts §2.2, §2.3) and the window is mapped on whatever the
    /// instance is showing.
    fn wait_until_ready(&self, live: &Live, address: &str) -> Result<(), Error> {
        poll_until(
            READY_TIMEOUT,
            || {
                let host = hypr::monitors()?;
                let instance = Ctl::Instance(&live.signature);
                let outputs = hypr::monitors_on(instance)?;
                let clients = hypr::clients_on(instance)?;
                Ok(capturable(
                    &host,
                    &outputs,
                    &clients,
                    &self.output,
                    &self.workspace,
                    address,
                ))
            },
            |observed| {
                format!(
                    "window {address} of agent desktop `{}` to become capturable (last observed: \
                     {observed})",
                    self.name
                )
            },
        )
    }

    /// The one-shot `fullscreen` rule is what makes the console fill the
    /// headless output; without it the host's gaps and reserved area shrink the
    /// agent desktop. The instance is still usable at its own size, so a drift
    /// is reported instead of repaired: repairing means touching host window
    /// state after §4.6 already declared the desktop untouched.
    fn warn_on_nested_size(&self, live: &Live) {
        let Ok(outputs) = hypr::monitors_on(Ctl::Instance(&live.signature)) else {
            return;
        };
        let Some(monitor) = outputs.first() else {
            return;
        };
        if output_is_configured(monitor, self.size) {
            return;
        }
        let _ = writeln!(
            std::io::stderr(),
            "hyprpilot: warning: agent desktop `{}` renders {}x{} at scale {} instead of the \
             requested {}x{} — the console window {} is probably not fullscreen on {}",
            self.name,
            monitor.width,
            monitor.height,
            monitor.scale,
            self.size[0],
            self.size[1],
            live.console_address,
            self.output,
        );
    }

    fn host_output(&self) -> Result<Option<hypr::Monitor>, Error> {
        Ok(hypr::monitors()?
            .into_iter()
            .find(|monitor| monitor.name == self.output))
    }

    fn marker(&self) -> Marker<'_> {
        Marker {
            session: self.name,
            instance: &self.nonce,
        }
    }

    /// Undoes what exists, in the order that never drops a live window onto the
    /// user's desktop, and reports the original failure alongside whatever the
    /// rollback could not undo.
    fn rolled_back(&self, error: Error, state: &Isolated, host: &HostSnapshot) -> Error {
        // The rollback is about to remove the session directory the failure message
        // points at, so the tail of the nested compositor's log is emitted here or
        // lost with it. Without it a start that fails leaves nothing to diagnose,
        // which is exactly how a console window that never mapped stayed
        // unexplained on the third gate run.
        warn_with_nested_log_tail(&self.dir);
        let restore = self.rollback(state, host);
        if restore.is_empty() {
            error
        } else {
            Error::Guarded {
                action: Some(Box::new(error)),
                restore,
            }
        }
    }

    /// The order is written out rather than drained blindly, because three of
    /// its steps deliberately stop and leave the state on disk for `teardown` to
    /// resume from — which a loop over the ledger cannot express.
    fn rollback(&self, state: &Isolated, host: &HostSnapshot) -> Vec<RestoreFailure> {
        if let Err(failure) = self.terminate(state) {
            // The console still lives on the headless output: removing that
            // output would drop the window onto the user's desktop, and the
            // state has to survive for `teardown`.
            return vec![failure];
        }
        let mut failures = Vec::new();
        // Fact §2.9: what the nested compositor left in $XDG_RUNTIME_DIR/hypr is
        // ours to remove, even when the start never got as far as recording a live
        // instance in its state — a `Pending` state has no signature to name that
        // directory with, so this is the only place that can. Hence before the
        // console wait, which is about another resource and may time out. The log
        // stays where the spawn redirected it.
        if let Some(signature) = instance_signature(&state.instance) {
            let (_, failure) = clear_instance_runtime(signature, None);
            failures.extend(failure);
        }
        // The compositor is dead, but the console is a *host* window and the host
        // reaps it asynchronously: `output remove` before that hands it to one of
        // the user's monitors for as long as it survives. Same wait, same reason,
        // as the teardown path.
        if let Err(error) = wait_console_reaped(&self.output, console_of(&state.instance)) {
            failures.push(RestoreFailure {
                what: "agent console window",
                expected: format!(
                    "reaped by the host before output {} is removed",
                    self.output
                ),
                actual: error.to_string(),
            });
            // The output stays where it is, and so does the state `teardown` needs.
            return failures;
        }
        // Only now: every undo that removes the output has to run after the
        // console it renders is gone. The snapshot is the cursor to warp back to
        // if `cursorpos` cannot be read at the last moment (fact §2.8).
        let unwound = unwind(&ledger::live_undo_effects(), &state.host, Some(host.cursor));
        // A rolled-back start returns no notes — its whole output is the failure
        // it reports — so what it took back, and what it could not, is said here
        // or nowhere.
        warn_unwound(&unwound);
        failures.extend(unwound.failures);
        if let Some(stopped) = unwound.stopped {
            // The state has to survive for `teardown`, so the session directory
            // below is left alone.
            failures.push(stopped);
            return failures;
        }
        if let Err(source) = fs::remove_dir_all(&self.dir) {
            failures.push(RestoreFailure {
                what: "session state",
                expected: format!("{} removed", self.dir.display()),
                actual: source.to_string(),
            });
        }
        failures
    }

    /// The same ladder as §6.2, not an immediate sweep: `dispatch exit` first,
    /// then a grace period, then `SIGTERM`, then `SIGKILL`. The grace is what
    /// lets the nested compositor exit on its own and clean up its runtime
    /// directory (fact §2.9); the marker sweep is what proves the desktop is
    /// gone, including a compositor spawned but never identified.
    fn terminate(&self, state: &Isolated) -> Result<(), RestoreFailure> {
        if let Some(signature) = instance_signature(&state.instance) {
            let _ = hypr::dispatch_on(Ctl::Instance(signature), &["exit"]);
        }
        terminate_marked(&self.marker()).map(|_| ())
    }
}

fn warn_unwound(unwound: &Unwound) {
    let mut stderr = std::io::stderr();
    for note in &unwound.notes {
        let _ = writeln!(stderr, "hyprpilot: rolled back: {note}");
    }
    for leak in &unwound.leaked {
        let _ = writeln!(stderr, "hyprpilot: warning: {leak}");
    }
}

/// The console of a start that got far enough to identify one: its address plus
/// the pid that owns it, since an address alone is a reusable pointer.
fn console_of(instance: &Instance) -> Option<Console<'_>> {
    match instance {
        Instance::Pending | Instance::Spawned { .. } => None,
        Instance::Live {
            pid,
            console_address,
            ..
        } => Some(Console {
            address: console_address,
            pid: *pid,
        }),
    }
}

/// The signature of a compositor that reached at least the `Spawned` stage —
/// the name of the runtime directory it leaves behind (fact §2.9) and the only
/// way to address it.
fn instance_signature(instance: &Instance) -> Option<&str> {
    match instance {
        Instance::Pending => None,
        Instance::Spawned { signature } | Instance::Live { signature, .. } => Some(signature),
    }
}

/// Unique to this start on this machine, and not a secret: it only has to be
/// something no inherited environment already carries, which the pid of the
/// process doing the start plus the current nanoseconds cover without a new
/// dependency.
fn instance_nonce() -> Result<String, Error> {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Tool {
            command: "clock".to_owned(),
            message: format!("the system clock is before the Unix epoch ({error})"),
        })?;
    Ok(format!("{}-{}", std::process::id(), since_epoch.as_nanos()))
}

/// What a teardown of an agent desktop brought down, and what it could not.
pub struct Teardown {
    pub notes: Vec<String>,
    pub failures: Vec<RestoreFailure>,
}

/// The registered identity of a nested compositor: the only pid teardown ever
/// signals, the runtime directory it leaves behind (fact §2.9), and the host
/// window it maps, whose disappearance gates the output removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Registered<'a> {
    signature: &'a str,
    pid: u32,
    console: &'a str,
    display: &'a str,
}

/// The nested compositor as the state knows it, at the two stages that have one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compositor<'a> {
    /// Registered in the host's instance table, its console not identified yet.
    /// The signature is enough for the two steps that matter — `dispatch exit`
    /// and the runtime directory it already leaves behind (fact §2.9) — and the
    /// marker sweep is what proves the desktop is gone. There is no pid to
    /// signal and no console whose reaping the output removal can wait on, so
    /// the wait falls back to the output being vacated.
    Spawned { signature: &'a str },
    /// Fully identified: a pid to signal, a console whose death gates the output
    /// removal, and a socket to unlink.
    Live(Registered<'a>),
}

impl<'a> Compositor<'a> {
    fn signature(self) -> &'a str {
        match self {
            Self::Spawned { signature } => signature,
            Self::Live(registered) => registered.signature,
        }
    }

    fn console(self) -> Option<Console<'a>> {
        match self {
            Self::Spawned { .. } => None,
            Self::Live(registered) => Some(Console {
                address: registered.console,
                pid: registered.pid,
            }),
        }
    }
}

/// What §6 has to undo, decided from the state alone so it can be asserted
/// without a compositor. Steps 4 (output and cursor) and 5 (state) are
/// unconditional and belong to `session::finish_teardown`.
#[derive(Debug, PartialEq, Eq)]
struct TeardownPlan<'a> {
    /// Window to close politely, inside the instance (step 1).
    close: Option<&'a str>,
    /// Compositor to exit, then whose runtime directory to clear (steps 2, 3).
    instance: Option<Compositor<'a>>,
}

fn teardown_plan(isolated: &Isolated) -> TeardownPlan<'_> {
    let instance = match &isolated.instance {
        Instance::Pending => None,
        Instance::Spawned { signature } => Some(Compositor::Spawned { signature }),
        Instance::Live {
            signature,
            pid,
            console_address,
            wayland_display,
        } => Some(Compositor::Live(Registered {
            signature,
            pid: *pid,
            console: console_address,
            display: wayland_display,
        })),
    };
    TeardownPlan {
        // A recorded window with no compositor to close it in is nothing to do.
        close: match (&instance, isolated.active_address.as_deref()) {
            (Some(_), Some(address)) => Some(address),
            _ => None,
        },
        instance,
    }
}

/// Steps 1 to 3 of §6, in the one order that is safe: the console window dies
/// before the output it renders into is removed, otherwise `hyprctl output
/// remove` drops it onto the user's desktop. Every step is idempotent — a window
/// already gone, a pid already dead, a directory already absent are successes —
/// so an `Instance::Pending` state cleans up as well as a live one.
pub fn teardown(session: &str, isolated: &Isolated) -> Result<Teardown, Error> {
    let plan = teardown_plan(isolated);
    let marker = Marker {
        session,
        instance: &isolated.instance_nonce,
    };
    let mut notes = Vec::new();
    let mut failures = Vec::new();

    // A console reaped on one of the user's workspaces (where `session show` put
    // it) makes the host refocus, and Hyprland's refocus re-warps the cursor —
    // the fact `guard` is built around. The output removal that follows reads the
    // cursor as it finds it, so that warp would outlive the teardown as if the
    // user had moved the mouse themselves. Sending the console back to the agent
    // workspace first is the whole fix: it then dies where nothing of the user's
    // looks, which is the path a hidden desktop already took.
    if let Some(Compositor::Live(instance)) = plan.instance {
        notes.extend(park_console_before_death(
            session,
            isolated,
            instance.console,
        ));
    }

    match plan.instance {
        None => notes.push("nested compositor was never spawned".to_owned()),
        Some(compositor) => {
            if let Some(address) = plan.close {
                notes.push(close_app(compositor.signature(), address));
            }
            match compositor {
                Compositor::Live(instance) => {
                    notes.extend(stop_registered(&instance, &marker)?);
                    let (unlinked, failure) =
                        clear_stale_socket(&session::runtime_root()?, instance.display);
                    notes.extend(unlinked);
                    failures.extend(failure);
                }
                // No pid recorded to escalate against and no socket name to
                // unlink: ask it to go, and let the marker sweep below prove it
                // did — it is the authority at every stage anyway.
                Compositor::Spawned { signature } => notes.push(
                    match hypr::dispatch_on(Ctl::Instance(signature), &["exit"]) {
                        Ok(()) => format!(
                            "nested compositor {signature} was asked to exit; it never named its \
                             console window"
                        ),
                        Err(error) => {
                            format!(
                                "nested compositor {signature} refused `dispatch exit` ({error})"
                            )
                        }
                    },
                ),
            }
            let (cleared, failure) = clear_instance_runtime(
                compositor.signature(),
                Some(&session::session_dir(session)?),
            );
            notes.extend(cleared);
            failures.extend(failure);
        }
    }

    // Whatever stage the state described, no process of this desktop may outlive
    // the teardown: both markers together are the authority, so a compositor
    // spawned but never recorded is caught here too. Its output is removed right
    // after this, hence the refusal rather than a note.
    match terminate_marked(&marker) {
        Ok(swept) => notes.extend(swept),
        Err(failure) => {
            return Err(Error::AgentDesktopAlive {
                session: session.to_owned(),
                detail: failure.actual,
            });
        }
    }
    notes.push("no process of the agent desktop left".to_owned());

    // The console is a *host* window, and the host reaps it asynchronously once
    // the compositor that owns it dies. Removing the output before that would hand
    // the window to one of the user's outputs for as long as it survives — which
    // is why this waits at every stage, including a state that never named a
    // console: a spawn the start could not identify still maps one.
    notes.push(wait_console_reaped(
        &isolated.output,
        plan.instance.and_then(Compositor::console),
    )?);

    Ok(Teardown { notes, failures })
}

/// Best effort by construction: everything this protects is cosmetic next to a
/// teardown that must destroy the desktop whatever the host answers, so a failure
/// is a note and never a refusal. `shown` is the state's memory of where the
/// console was put, so the question is asked of the host instead.
fn park_console_before_death(session: &str, isolated: &Isolated, console: &str) -> Vec<String> {
    let Ok(current) = host_console(console, session) else {
        return vec![format!(
            "console window {console} could not be read before the kill; left where it is"
        )];
    };
    if visibility(&current.workspace.name, &isolated.workspace) == Visibility::Hidden {
        return Vec::new();
    }
    match hypr::dispatch(&[
        "movetoworkspacesilent",
        &format!(
            "{},address:{console}",
            session::workspace_selector(&isolated.workspace)
        ),
    ]) {
        Ok(()) => vec![format!(
            "shown console window {console} put back on {} before its compositor died",
            isolated.workspace
        )],
        Err(error) => vec![format!(
            "shown console window {console} could not be put back on {} ({error}); the user's \
             cursor may end up where the host refocused",
            isolated.workspace
        )],
    }
}

/// A console window as the state knows it. The pid is part of the identity
/// because Hyprland addresses are recycled pointers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Console<'a> {
    address: &'a str,
    pid: u32,
}

/// Bounded, and a timeout aborts rather than removing an output a window still
/// sits on — the same rule the shared teardown follows before it removes its own
/// output. A window already gone is an immediate success (§6.5).
///
/// Identity is the recorded address when there is one, and the headless output
/// itself when there is not: whatever the host still composites there is what
/// `output remove` would relocate onto one of the user's monitors.
fn wait_console_reaped(output: &str, console: Option<Console<'_>>) -> Result<String, Error> {
    poll_until(
        session::WINDOW_PLACE_TIMEOUT,
        || {
            let clients = hypr::clients()?;
            Ok(match console {
                Some(console) => console_reaped(&clients, console),
                None => output_vacated(&hypr::monitors()?, &clients, output),
            })
        },
        |observed| {
            format!(
                "the host to reap the console window of output {output} now that its compositor is \
                 dead (last observed: {observed}); the output stays until then"
            )
        },
    )
}

/// Every process of this desktop is already gone when this runs, so a window
/// still answering at the console's address whose pid is not the compositor's is
/// somebody else's window that inherited a recycled pointer — not a console to
/// wait for. Waiting for it anyway is how a teardown ends up refusing for ever
/// and leaving the output behind.
fn console_reaped(clients: &[hypr::Client], console: Console<'_>) -> Result<String, String> {
    let Some(client) = find_client(clients, console.address) else {
        return Ok(format!(
            "console window {} is gone from the host",
            console.address
        ));
    };
    if u32::try_from(client.pid) != Ok(console.pid) {
        return Ok(format!(
            "address {} now belongs to pid {} (`{}`), not the dead nested compositor (pid {})",
            console.address, client.pid, client.title, console.pid
        ));
    }
    Err(format!("still on workspace {}", client.workspace.name))
}

/// The same wait for a state that never recorded a console: identity is then the
/// monitor a window reports, and the agent's headless output has to be empty
/// before it goes.
fn output_vacated(
    monitors: &[hypr::Monitor],
    clients: &[hypr::Client],
    output: &str,
) -> Result<String, String> {
    let Some(monitor) = monitors.iter().find(|monitor| monitor.name == output) else {
        return Ok(format!("output {output} is already gone"));
    };
    let occupants = clients
        .iter()
        .filter(|client| client.monitor == monitor.id)
        .map(|client| format!("{} (`{}`)", client.address, client.title))
        .collect::<Vec<_>>();
    if occupants.is_empty() {
        return Ok(format!("no window is left on output {output}"));
    }
    Err(format!("{} still there", occupants.join(", ")))
}

/// Step 1 of §6: politeness only, addressed to the instance and never to the
/// host. `dispatch exit` takes the desktop down anyway, so nothing here fails
/// the teardown; the outcome is reported as a note.
fn close_app(signature: &str, address: &str) -> String {
    let ctl = Ctl::Instance(signature);
    let clients = match hypr::clients_on(ctl) {
        Ok(clients) => clients,
        Err(error) => return format!("could not list the agent desktop's windows ({error})"),
    };
    if find_client(&clients, address).is_none() {
        return format!("window {address} was already gone from the agent desktop");
    }
    match hypr::dispatch_on(ctl, &["closewindow", &format!("address:{address}")]) {
        Ok(()) => close_settled(ctl, address),
        Err(error) => format!("window {address} refused to close ({error})"),
    }
}

/// A well-behaved app unmaps within a few frames; one that wants longer (a
/// modal, a prompt) gets taken down by `dispatch exit` instead of blocking here.
/// The same bounded loop as everywhere else — only the outcome differs: a note,
/// never a failure.
fn close_settled(ctl: Ctl<'_>, address: &str) -> String {
    poll_until(
        POLITE_CLOSE_TIMEOUT,
        || {
            Ok(find_client(&hypr::clients_on(ctl)?, address).map_or_else(
                || Ok(format!("closed window {address} in the agent desktop")),
                |window| Err(format!("still mapped on {}", window.workspace.name)),
            ))
        },
        |observed| format!("window {address} to leave the agent desktop ({observed})"),
    )
    .unwrap_or_else(|error| format!("asked window {address} to close: {error}"))
}

/// The effects of step 2, injected so the escalation ladder is testable without
/// a compositor or a real process.
struct Exit<'a> {
    session: &'a str,
    label: String,
    request: &'a dyn Fn() -> Result<(), Error>,
    alive: &'a dyn Fn() -> bool,
    signal: &'a dyn Fn(&'static str) -> Result<(), Error>,
}

fn stop_registered(instance: &Registered<'_>, marker: &Marker<'_>) -> Result<Vec<String>, Error> {
    let ctl = Ctl::Instance(instance.signature);
    let pid = instance.pid;
    stop_instance(
        &Exit {
            session: marker.session,
            label: format!("nested compositor {} (pid {pid})", instance.signature),
            request: &|| hypr::dispatch_on(ctl, &["exit"]),
            // The markers are both liveness and identity: a recycled pid carries
            // neither, so it is never signalled.
            alive: &|| pid_carries_marker(pid, marker),
            signal: &|signal| session::signal_process(pid, signal),
        },
        EXIT_ESCALATION,
    )
}

/// Step 2 of §6: `dispatch exit`, bounded wait for the pid to die, then
/// `SIGTERM`, then `SIGKILL`. Each escalation lands in the notes.
fn stop_instance(steps: &Exit<'_>, ladder: session::Escalation) -> Result<Vec<String>, Error> {
    let label = &steps.label;
    let mut notes = Vec::new();
    if !(steps.alive)() {
        notes.push(format!("{label} was already dead"));
        return Ok(notes);
    }
    if let Err(error) = (steps.request)() {
        notes.push(format!("{label} refused `dispatch exit` ({error})"));
    }

    let started = Instant::now();
    let mut sent: Option<&'static str> = None;
    loop {
        if !(steps.alive)() {
            notes.push(sent.map_or_else(
                || format!("{label} exited on `dispatch exit`"),
                |signal| format!("{label} died after SIG{signal}"),
            ));
            return Ok(notes);
        }
        match ladder.step(started.elapsed()) {
            session::Step::Signal(signal) if sent != Some(signal) => {
                notes.push(match (steps.signal)(signal) {
                    Ok(()) => format!("{label}: SIG{signal} sent"),
                    Err(error) => format!("{label}: SIG{signal} failed ({error})"),
                });
                sent = Some(signal);
            }
            session::Step::Wait | session::Step::Signal(_) => {}
            session::Step::GiveUp => {
                return Err(Error::AgentDesktopAlive {
                    session: steps.session.to_owned(),
                    detail: format!("{label} survived `dispatch exit`, SIGTERM and SIGKILL"),
                });
            }
        }
        thread::sleep(ladder.poll);
    }
}

/// Fact §2.9: the nested compositor leaves `$XDG_RUNTIME_DIR/hypr/<sig>/` behind
/// after `dispatch exit`, so step 3 removes it explicitly. The signature comes
/// from a state file, and it ends up in a recursive removal: it has to be a
/// single directory name.
fn instance_dir(instances: &Path, signature: &str) -> Result<PathBuf, Error> {
    Ok(instances.join(plain_signature(signature)?))
}

/// The signature is read back from a state file and ends up in a recursive
/// removal, so it has to be a single directory name.
fn plain_signature(signature: &str) -> Result<&str, Error> {
    if signature.is_empty()
        || signature == "."
        || signature == ".."
        || signature.contains('/')
        || signature.contains('\0')
    {
        return Err(Error::Invalid {
            what: "instance signature",
            value: signature.to_owned(),
            hint: "expected a single directory name under $XDG_RUNTIME_DIR/hypr".to_owned(),
        });
    }
    Ok(signature)
}

/// Step 3 of §6: keep the compositor's own log, then remove the runtime
/// directory it left behind (fact §2.9). A directory that cannot go is reported
/// rather than propagated: holding up the output removal over it would leave the
/// user with a compositing headless output instead of an empty directory.
///
/// `keep_log_in` is `None` for a rolled-back start, whose session directory is
/// removed a few lines later: copying the log into it would only delete it again.
fn clear_instance_runtime(
    signature: &str,
    keep_log_in: Option<&Path>,
) -> (Vec<String>, Option<RestoreFailure>) {
    let dir = match instances_dir().and_then(|instances| instance_dir(&instances, signature)) {
        Ok(dir) => dir,
        Err(error) => {
            return (
                Vec::new(),
                Some(RestoreFailure {
                    what: "nested runtime directory",
                    expected: format!("$XDG_RUNTIME_DIR/hypr/{signature} removed"),
                    actual: error.to_string(),
                }),
            );
        }
    };
    clear_runtime_dir(&dir, keep_log_in)
}

/// libwayland only unlinks its socket on a clean exit, so a nested compositor
/// killed with `SIGKILL` leaves `wayland-<n>` and its lock behind (§2.9 applies
/// to the socket as much as to the instance directory).
///
/// Refuses to unlink a socket something still listens on: the recorded name is
/// ours, but a stale name could have been taken over by another compositor
/// between the kill and this call, and unlinking that one would cut its clients
/// off.
fn clear_stale_socket(runtime: &Path, display: &str) -> (Vec<String>, Option<RestoreFailure>) {
    let socket = runtime.join(display);
    if !socket.exists() {
        return (vec![format!("socket {display} already gone")], None);
    }
    if UnixStream::connect(&socket).is_ok() {
        return (
            Vec::new(),
            Some(RestoreFailure {
                what: "nested Wayland socket",
                expected: format!("{display} unlinked"),
                actual: "something still accepts connections on it".to_owned(),
            }),
        );
    }
    let mut notes = Vec::new();
    let mut failure = None;
    for path in [socket, runtime.join(format!("{display}.lock"))] {
        match fs::remove_file(&path) {
            Ok(()) => notes.push(format!("removed {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                failure = Some(RestoreFailure {
                    what: "nested Wayland socket",
                    expected: format!("{} removed", path.display()),
                    actual: error.to_string(),
                });
            }
        }
    }
    (notes, failure)
}

fn clear_runtime_dir(
    dir: &Path,
    keep_log_in: Option<&Path>,
) -> (Vec<String>, Option<RestoreFailure>) {
    let mut notes = keep_log_in
        .map(|session_dir| keep_instance_log(dir, session_dir))
        .into_iter()
        .collect::<Vec<_>>();
    match remove_instance_dir(dir) {
        Ok(note) => {
            notes.push(note);
            (notes, None)
        }
        Err(failure) => (notes, Some(failure)),
    }
}

/// The compositor's own log is the only diagnostic of a nested crash and it
/// lives in the directory step 3 removes, so it is copied next to the session's
/// own log first: that copy is what a teardown failing after this point leaves
/// behind.
/// Enough to carry a compositor's own explanation of why it never got as far as
/// mapping its console, and short enough to stay readable in a command's stderr.
const NESTED_LOG_TAIL_LINES: usize = 20;

fn warn_with_nested_log_tail(dir: &Path) {
    let path = dir.join(NESTED_LOG_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let tail: Vec<&str> = text
        .lines()
        .rev()
        .take(NESTED_LOG_TAIL_LINES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if tail.is_empty() {
        return;
    }
    let mut stderr = std::io::stderr();
    let _ = writeln!(
        stderr,
        "hyprpilot: warning: last {} lines of {} before the rollback removed it:",
        tail.len(),
        path.display()
    );
    for line in tail {
        let _ = writeln!(stderr, "hyprpilot:   {line}");
    }
}

fn keep_instance_log(instance_dir: &Path, session_dir: &Path) -> String {
    let source = instance_dir.join(NESTED_LOG_FILE);
    if !source.is_file() {
        return format!("no nested log at {}", source.display());
    }
    let destination = session_dir.join(INSTANCE_LOG_FILE);
    match fs::copy(&source, &destination) {
        Ok(_) => format!("kept the nested log as {}", destination.display()),
        Err(error) => format!("could not keep {} ({error})", source.display()),
    }
}

fn remove_instance_dir(dir: &Path) -> Result<String, RestoreFailure> {
    match fs::remove_dir_all(dir) {
        Ok(()) => Ok(format!("removed {}", dir.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(format!("{} already absent", dir.display()))
        }
        Err(error) => Err(RestoreFailure {
            what: "nested runtime directory",
            expected: format!("{} removed", dir.display()),
            actual: error.to_string(),
        }),
    }
}

fn output_name(session: &str) -> String {
    format!("{}-{session}", session::OUTPUT_NAME)
}

fn workspace_name(session: &str) -> String {
    format!("agent-{session}")
}

fn agent_session_marker() -> Option<String> {
    env::var_os(AGENT_SESSION_ENV).map(|value| value.to_string_lossy().into_owned())
}

/// The precondition of **every** command, not just of a start. Inside an agent
/// desktop `hyprctl` answers for the nested compositor, so a headless output
/// created there stays 0x0 (fact §2.7) and captures of it are silently blank;
/// and every process around carries this desktop's session marker, the caller's
/// own shell included, so a sweep run from in here would take it down.
pub fn refuse_when_nested() -> Result<(), Error> {
    refuse_nested_marker(agent_session_marker().as_deref())
}

fn refuse_nested_marker(marker: Option<&str>) -> Result<(), Error> {
    marker.map_or(Ok(()), |session| {
        Err(Error::NestedRefused {
            session: session.to_owned(),
        })
    })
}

/// An output named after this session is a leftover from an earlier agent
/// desktop, never a resource to reuse (§4.2).
fn ensure_output_absent(
    monitors: &[hypr::Monitor],
    output: &str,
    session: &str,
) -> Result<(), Error> {
    if monitors.iter().any(|monitor| monitor.name == output) {
        return Err(Error::AgentOutputExists {
            output: output.to_owned(),
            session: session.to_owned(),
        });
    }
    Ok(())
}

/// A fresh headless output brings a workspace of its own, and that is the only
/// kind this start may rename. Anything that already existed when the snapshot
/// was taken is the user's — an empty workspace they keep as a scratch pad is
/// still theirs, and nothing would give the name back at teardown (§12.1).
fn renameable(
    host: &HostSnapshot,
    clients: &[hypr::Client],
    workspace: &str,
) -> Result<(), String> {
    let occupants = workspace_occupants(clients, workspace);
    if !occupants.is_empty() {
        return Err(format!("holds {}", occupants.join(", ")));
    }
    if let Some((output, _)) = host
        .workspaces
        .iter()
        .find(|(_, active)| *active == workspace)
    {
        return Err(format!("it was visible on {output} a moment ago"));
    }
    if host.workspace_names.contains(workspace) {
        return Err(
            "it already existed before this start created its output, so it is the user's"
                .to_owned(),
        );
    }
    Ok(())
}

fn workspace_occupants(clients: &[hypr::Client], workspace: &str) -> Vec<String> {
    clients
        .iter()
        .filter(|client| client.workspace.name == workspace)
        .map(|client| format!("{} (`{}`)", client.address, client.title))
        .collect()
}

fn output_is_configured(monitor: &hypr::Monitor, size: [u32; 2]) -> bool {
    exact(monitor.width, f64::from(size[0]))
        && exact(monitor.height, f64::from(size[1]))
        && exact(monitor.scale, 1.0)
}

/// The mode-set either applied or it did not; there is no tolerance to grant.
fn exact(actual: f64, expected: f64) -> bool {
    actual.total_cmp(&expected) == Ordering::Equal
}

fn console_pid(console: &hypr::Client) -> Result<u32, Error> {
    u32::try_from(console.pid).map_err(|_| Error::Tool {
        command: "hyprctl clients".to_owned(),
        message: format!(
            "console window {} reports invalid pid {}",
            console.address, console.pid
        ),
    })
}

fn host_snapshot() -> Result<HostSnapshot, Error> {
    Ok(HostSnapshot {
        workspaces: active_workspaces(&hypr::monitors()?),
        workspace_names: hypr::workspace_names()?,
        active_window: hypr::active_window()?.map(|window| window.address),
        cursor: hypr::cursor_pos()?,
    })
}

fn active_workspaces(monitors: &[hypr::Monitor]) -> BTreeMap<String, String> {
    monitors
        .iter()
        .map(|monitor| (monitor.name.clone(), monitor.active_workspace.name.clone()))
        .collect()
}

/// §4.6: any deviation of the user's desktop fails the start. Outputs this
/// start created are ignored — only what existed before is compared.
fn deviation(before: &HostSnapshot, after: &HostSnapshot) -> Option<Error> {
    for (output, workspace) in &before.workspaces {
        let deviated = |what, actual: String| {
            Some(Error::HostDeviation {
                what,
                expected: format!("{workspace} on {output}"),
                actual,
            })
        };
        match after.workspaces.get(output) {
            None => return deviated("host output", format!("{output} is gone")),
            Some(current) if current != workspace => {
                return deviated("active workspace", format!("{current} on {output}"));
            }
            Some(_) => {}
        }
    }
    if before.active_window == after.active_window {
        return None;
    }
    Some(Error::HostDeviation {
        what: "focused window",
        expected: focus_label(before.active_window.as_deref()),
        actual: focus_label(after.active_window.as_deref()),
    })
}

fn focus_label(address: Option<&str>) -> String {
    address.map_or_else(
        || "no focused window".to_owned(),
        |address| format!("address:{address}"),
    )
}

fn check_host(before: &HostSnapshot) -> Result<(), Error> {
    deviation(before, &host_snapshot()?).map_or(Ok(()), Err)
}

fn host_keymap() -> Result<Keymap, Error> {
    keymap_of(&hypr::devices()?)
}

fn keymap_of(devices: &hypr::Devices) -> Result<Keymap, Error> {
    let main = devices
        .keyboards
        .iter()
        .find(|keyboard| keyboard.main)
        .ok_or_else(|| Error::Tool {
            command: "hyprctl devices".to_owned(),
            message: "no main keyboard reported — cannot inherit the host keymap".to_owned(),
        })?;
    Ok(Keymap {
        rules: main.rules.clone(),
        model: main.model.clone(),
        layout: main.layout.clone(),
        variant: main.variant.clone(),
        options: main.options.clone(),
    })
}

/// §4.4: animations off, uniform wallpaper, no gaps or borders, host keymap, no
/// `exec-once`, and no portal rule (the portal probe verdict is UNSUPPORTED).
pub fn nested_config(session: &str, size: [u32; 2], keymap: &Keymap) -> String {
    let [width, height] = size;
    let header = format!(
        "# hyprpilot agent desktop `{session}` — generated at session start, do not edit.\n\
         # It renders into the host output {}.\n\
         \n\
         monitor = , {width}x{height}@60, 0x0, 1\n",
        output_name(session)
    );
    let input = format!(
        "input {{\n\
         \x20   kb_rules = {}\n\
         \x20   kb_model = {}\n\
         \x20   kb_layout = {}\n\
         \x20   kb_variant = {}\n\
         \x20   kb_options = {}\n\
         }}\n",
        keymap.rules, keymap.model, keymap.layout, keymap.variant, keymap.options
    );
    [header.as_str(), LAYOUT_BLOCKS, input.as_str(), DEBUG_BLOCK].join("\n")
}

fn ensure_nested_binary() -> Result<(), Error> {
    if nested_binary_present() {
        return Ok(());
    }
    Err(Error::Tool {
        command: NESTED_BINARY.to_owned(),
        message: format!(
            "{NESTED_BINARY} not found on PATH — an agent desktop is a nested Hyprland"
        ),
    })
}

pub fn nested_binary_present() -> bool {
    session::binary_on_path(NESTED_BINARY)
}

/// Hyprland hands the command to `sh -c`, so the paths are quoted and a path
/// that could close a quote is refused rather than escaped. Both markers are
/// injected here: everything the nested compositor goes on to spawn inherits
/// them, which is what makes the pair an identity for this desktop's processes.
fn spawn_command(
    marker: &Marker<'_>,
    workspace: &str,
    config: &Path,
    log: &Path,
) -> Result<String, Error> {
    let config = shell_path(config)?;
    let log = shell_path(log)?;
    Ok(format!(
        "[workspace name:{workspace} silent; noinitialfocus; fullscreen] \
         env {AGENT_SESSION_ENV}={session} {AGENT_INSTANCE_ENV}={instance} {NESTED_BINARY} -c \
         '{config}' > '{log}' 2>&1",
        session = marker.session,
        instance = marker.instance,
    ))
}

fn shell_path(path: &Path) -> Result<&str, Error> {
    let invalid = |hint: &str| Error::Invalid {
        what: "agent desktop path",
        value: path.display().to_string(),
        hint: hint.to_owned(),
    };
    let text = path
        .to_str()
        .ok_or_else(|| invalid("expected valid UTF-8"))?;
    if text.contains('\'') || text.contains('\n') {
        return Err(invalid("must not contain a quote or a newline"));
    }
    Ok(text)
}

/// Where every compositor keeps its instance directory, the nested ones
/// included: teardown removes the one belonging to this session (fact §2.9).
pub fn instances_dir() -> Result<PathBuf, Error> {
    Ok(session::runtime_root()?.join("hypr"))
}

/// Ours is the instance whose process carries both markers of this start. The
/// per-start nonce makes that unique, so two matches mean something is lying and
/// the start refuses to adopt a stranger's compositor rather than guess.
fn marked_instance<'a>(
    instances: &'a [hypr::InstanceInfo],
    ours: &dyn Fn(i32) -> Result<bool, Error>,
) -> Result<Option<&'a hypr::InstanceInfo>, Error> {
    let mut found: Option<&hypr::InstanceInfo> = None;
    for instance in instances {
        if !ours(instance.pid)? {
            continue;
        }
        if let Some(other) = found {
            return Err(Error::Tool {
                command: "hyprctl instances".to_owned(),
                message: format!(
                    "two live instances carry the markers of this start ({} pid {}, {} pid {}) — \
                     refusing to guess which one it spawned; teardown and retry",
                    other.instance, other.pid, instance.instance, instance.pid
                ),
            });
        }
        found = Some(instance);
    }
    Ok(found)
}

/// The console carries the nested compositor's class (fact §2.5) and its process
/// carries both markers of this start; the title is never part of the identity,
/// and neither is the address. Addresses are recycled pointers, so a console that
/// inherited the address of a window closed since the snapshot must not be
/// discarded: `before` only annotates an ambiguity, it never filters.
fn select_console<'a>(
    clients: &'a [hypr::Client],
    before: &BTreeSet<String>,
    ours: &dyn Fn(i32) -> Result<bool, Error>,
) -> Result<Option<&'a hypr::Client>, Error> {
    let mut console: Option<&hypr::Client> = None;
    for client in clients {
        if client.class != CONSOLE_CLASS {
            continue;
        }
        if !ours(client.pid)? {
            continue;
        }
        if let Some(other) = console {
            let candidate = |client: &hypr::Client| {
                format!(
                    "{} (pid {}{})",
                    client.address,
                    client.pid,
                    if before.contains(&client.address) {
                        ", at an address that predates this start"
                    } else {
                        ""
                    }
                )
            };
            return Err(Error::Tool {
                command: "hyprctl clients".to_owned(),
                message: format!(
                    "two console windows carry {AGENT_SESSION_ENV} and {AGENT_INSTANCE_ENV} for \
                     this start: {} and {} — teardown and retry",
                    candidate(other),
                    candidate(client)
                ),
            });
        }
        console = Some(client);
    }
    Ok(console)
}

fn find_client<'a>(clients: &'a [hypr::Client], address: &str) -> Option<&'a hypr::Client> {
    clients.iter().find(|client| client.address == address)
}

/// Capturability of an agent desktop window, host side and instance side.
fn capturable(
    host: &[hypr::Monitor],
    outputs: &[hypr::Monitor],
    clients: &[hypr::Client],
    output: &str,
    workspace: &str,
    address: &str,
) -> Result<(), String> {
    host_frames(host, output, workspace)?;
    window_frame(outputs, clients, address, AGENT_DESKTOP).map(|_| ())
}

/// Host side of capturability, and the §2.2 invariant of the whole design: the
/// nested compositor only keeps receiving frame callbacks while the host
/// composites its console window, which is what an active agent workspace on the
/// headless output guarantees.
fn host_frames(host: &[hypr::Monitor], output: &str, workspace: &str) -> Result<(), String> {
    let Some(headless) = host.iter().find(|monitor| monitor.name == output) else {
        return Err(format!("host output {output} is absent"));
    };
    if headless.active_workspace.name != workspace {
        return Err(format!(
            "workspace {} is active on {output}, not {workspace} — the nested compositor would \
             stop receiving frames",
            headless.active_workspace.name
        ));
    }
    if !headless.special_workspace.is_empty() {
        return Err(format!(
            "special workspace {} occludes {output}",
            headless.special_workspace
        ));
    }
    Ok(())
}

/// Whether a compositor is actually showing a window of its own: mapped, on the
/// active workspace of the output it reports, with no special workspace over it.
/// Returns what a capture needs to frame it.
///
/// `place` names the compositor being asked, because both of them are: an
/// instance for the window a capture frames, and the host for the console window
/// a *shown* agent desktop depends on (fact §2.2).
fn window_frame<'a>(
    outputs: &'a [hypr::Monitor],
    clients: &'a [hypr::Client],
    address: &str,
    place: &str,
) -> Result<(&'a hypr::Client, &'a hypr::Monitor), String> {
    let Some(window) = find_client(clients, address) else {
        return Err(format!("window {address} is gone from {place}"));
    };
    if window.size[0] <= 0 || window.size[1] <= 0 {
        return Err(format!(
            "window {address} is not mapped (size {:?})",
            window.size
        ));
    }
    let Some(monitor) = outputs.iter().find(|monitor| monitor.id == window.monitor) else {
        return Err(format!(
            "window {address} reports monitor {} which {place} does not have",
            window.monitor
        ));
    };
    if monitor.active_workspace.name != window.workspace.name {
        return Err(format!(
            "window {address} sits on workspace {} while {place} shows {}",
            window.workspace.name, monitor.active_workspace.name
        ));
    }
    if !monitor.special_workspace.is_empty() {
        return Err(format!(
            "special workspace {} occludes {place}",
            monitor.special_workspace
        ));
    }
    Ok((window, monitor))
}

/// The agent desktop, as `window_frame` names it when the question is about a
/// window inside the instance.
const AGENT_DESKTOP: &str = "the agent desktop";
/// The user's session, as `window_frame` names it when the question is about the
/// console window of a shown agent desktop.
const HOST: &str = "the host";

/// The nested compositor an isolated command talks to, borrowed from the state
/// that recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveInstance<'a> {
    pub signature: &'a str,
    pub wayland_display: &'a str,
    pub pid: u32,
    /// Host-side console window; only `show`/`hide` and teardown act on it.
    pub console: &'a str,
}

impl<'a> LiveInstance<'a> {
    /// Tied to the state's lifetime, not to the borrow of this value, so a
    /// caller can keep addressing the instance after dropping it.
    pub fn ctl(self) -> Ctl<'a> {
        Ctl::Instance(self.signature)
    }
}

/// The one gate every isolated command goes through (§5), so a dead agent
/// desktop fails here instead of timing out on an instance that will never
/// answer — or worse, falling back to the user's own desktop. `teardown` is the
/// one command that deliberately skips it: it is what cleans a dead desktop up.
pub fn live_instance<'a>(session: &str, isolated: &'a Isolated) -> Result<LiveInstance<'a>, Error> {
    live_instance_in(Path::new("/proc"), session, isolated)
}

fn live_instance_in<'a>(
    proc_root: &Path,
    session: &str,
    isolated: &'a Isolated,
) -> Result<LiveInstance<'a>, Error> {
    let unfinished = |what: &str| {
        Err(Error::AgentDesktopUnready {
            session: session.to_owned(),
            reason: format!(
                "{what} (`session start --isolated` did not finish) — run `hyprpilot --session \
                 {session} teardown` and start it again"
            ),
        })
    };
    let (signature, wayland_display, pid, console_address) = match &isolated.instance {
        Instance::Live {
            signature,
            wayland_display,
            pid,
            console_address,
        } => (signature, wayland_display, pid, console_address),
        Instance::Pending => return unfinished("its nested compositor was never spawned"),
        Instance::Spawned { .. } => {
            return unfinished(
                "its nested compositor was spawned but never mapped a console window this start \
                 could identify",
            );
        }
    };
    // The markers are liveness *and* identity: a dead pid has no readable
    // environment and a recycled one carries neither marker of this start, so no
    // command can address a stranger's process.
    let marker = Marker {
        session,
        instance: &isolated.instance_nonce,
    };
    if !pid_carries_marker_in(proc_root, *pid, &marker) {
        return Err(Error::AgentDesktopDead {
            session: session.to_owned(),
            signature: signature.clone(),
            pid: *pid,
        });
    }
    Ok(LiveInstance {
        signature,
        wayland_display,
        pid: *pid,
        console: console_address,
    })
}

/// The window an isolated command acts on: the one the start recorded, or the
/// one `target` last selected.
// TODO: record the window's `stableId` next to its address, as the shared window
// table does since schema v4, and check it here. A nested compositor recycles
// addresses like any other, so a window that closes can hand its address to
// another window of the app under test, and input then reaches the wrong one.
// Bounded to the agent's own desktop, which is why it is not part of the
// identity fix on the shared path.
pub fn recorded_window<'a>(session: &str, isolated: &'a Isolated) -> Result<&'a str, Error> {
    isolated
        .active_address
        .as_deref()
        .ok_or_else(|| Error::AgentDesktopUnready {
            session: session.to_owned(),
            reason: format!(
                "no window is recorded for it, its app never appeared — run `hyprpilot --session \
                 {session} teardown` and start it again"
            ),
        })
}

/// What a capture of an agent desktop acts on (§5): the socket grim has to talk
/// to, the window inside the nested layout, and the output that frames it.
pub struct AgentCapture {
    pub wayland_display: String,
    pub window: hypr::Client,
    pub monitor: hypr::Monitor,
    /// Host-side console window, so a capture that blocks while the desktop is
    /// shown can name the window whose compositing it depended on.
    pub console: String,
}

/// Resolves a capture inside an agent desktop, refusing a desktop that cannot
/// produce a frame instead of letting grim block on screencopy.
pub fn capture_target(session: &str, isolated: &Isolated) -> Result<AgentCapture, Error> {
    let unready = |reason: String| Error::AgentDesktopUnready {
        session: session.to_owned(),
        reason,
    };
    let instance = live_instance(session, isolated)?;
    let address = recorded_window(session, isolated)?;
    // Fact §2.2: a console window the host stopped compositing freezes the nested
    // compositor, and screencopy then blocks for ever. The invariant holds in a
    // different place either side of `session show`, and `shown` is only the
    // state's memory of where the console was put — never an observation — so both
    // sides are read from the host, every time.
    if let Err(observed) = frames_probe(&frame_site(isolated, instance.console))? {
        return Err(unready(frames_reason(
            session,
            &frame_site(isolated, instance.console),
            &observed,
        )));
    }

    let ctl = instance.ctl();
    let outputs = hypr::monitors_on(ctl)?;
    let clients = hypr::clients_on(ctl)?;
    let (window, monitor) =
        window_frame(&outputs, &clients, address, AGENT_DESKTOP).map_err(unready)?;
    Ok(AgentCapture {
        wayland_display: instance.wayland_display.to_owned(),
        window: window.clone(),
        monitor: monitor.clone(),
        console: instance.console.to_owned(),
    })
}

/// Where the console window has to be composited for the nested compositor to
/// keep receiving frame callbacks (fact §2.2). `session show` moves it, so it
/// moves the question every capture and every diagnosis has to ask.
pub enum FrameSite<'a> {
    /// Hidden: on the agent workspace, which has to be the active one of the
    /// agent's own headless output.
    Headless { output: &'a str, workspace: &'a str },
    /// Shown: a host window among the user's, which has to be one the host is
    /// actually showing.
    Shown { console: &'a str },
}

/// Reads the site off the state, never the invariant: `shown` records where the
/// console was *put*, and the user has been free to switch workspace since.
pub fn frame_site<'a>(isolated: &'a Isolated, console: &'a str) -> FrameSite<'a> {
    if isolated.shown {
        FrameSite::Shown { console }
    } else {
        FrameSite::Headless {
            output: &isolated.output,
            workspace: &isolated.workspace,
        }
    }
}

/// One host-side observation of the §2.2 invariant, wherever the console
/// currently lives.
fn frames_probe(site: &FrameSite<'_>) -> Probe<()> {
    match *site {
        FrameSite::Headless { output, workspace } => {
            Ok(host_frames(&hypr::monitors()?, output, workspace))
        }
        // A shown console is a host window like any other: mapped, on the active
        // workspace of its monitor, nothing special over it — which is exactly
        // what `window_frame` checks, asked of the host this time.
        FrameSite::Shown { console } => {
            let monitors = hypr::monitors()?;
            let clients = hypr::clients()?;
            Ok(window_frame(&monitors, &clients, console, HOST).map(|_| ()))
        }
    }
}

/// Fact §2.2, spelled out for the user: the one documented cause of a frozen
/// agent desktop, named where it actually applies.
pub fn frames_reason(session: &str, site: &FrameSite<'_>, observed: &str) -> String {
    match *site {
        FrameSite::Headless { output, workspace } => {
            frozen_reason(session, output, workspace, observed)
        }
        FrameSite::Shown { console } => format!(
            "agent desktop `{session}` is shown, so its console window {console} only receives \
             frame callbacks while the host composites it where it stands, and every capture \
             blocks once it stops ({observed}) — switch back to the workspace it sits on, or run \
             `hyprpilot --session {session} session hide` to put it back on its headless output"
        ),
    }
}

/// The headless half of `frames_reason`, with the documented host-side fallback.
pub fn frozen_reason(session: &str, host_output: &str, workspace: &str, observed: &str) -> String {
    format!(
        "the nested compositor only receives frame callbacks while workspace {workspace} is the \
         active one on host output {host_output}, and every capture blocks once it stops \
         ({observed}) — capture the host side with `grim -o {host_output}` as a documented \
         fallback, or run `hyprpilot --session {session} teardown` and start the agent desktop \
         again"
    )
}

/// Reads the host side of the §2.2 invariant, so a blocked capture reports what
/// was observed rather than what is likely.
pub fn frames_observation(site: &FrameSite<'_>) -> String {
    match frames_probe(site) {
        Ok(Err(observed)) => observed,
        Ok(Ok(())) => match *site {
            FrameSite::Headless { output, workspace } => format!(
                "workspace {workspace} is still active on {output}, so the block is elsewhere"
            ),
            FrameSite::Shown { console } => format!(
                "console window {console} is still mapped on the workspace it was shown on, so the \
                 block is elsewhere"
            ),
        },
        Err(error) => format!("reading the host failed: {error}"),
    }
}

/// `target` in an agent desktop (§5): the exact matcher of shared mode, run
/// against the clients of the instance, then `focuswindow` inside it. No parking
/// and no disposition — the parked workspace hides the *user's* other windows
/// and the dispositions give them back at teardown, and an agent desktop has
/// neither a user to hide windows from nor anything that outlives its teardown.
pub fn target(
    session: &str,
    path: &Path,
    isolated: &mut Isolated,
    criteria: &session::Criteria<'_>,
    untracked: bool,
    wait: Option<Duration>,
    on_teardown: Option<session::Disposition>,
) -> Result<String, Error> {
    refuse_disposition(on_teardown)?;
    refuse_untracked(untracked)?;
    let signature = live_instance(session, isolated)?.signature.to_owned();
    let window = wait_for_instance_window(&signature, session, criteria, wait)?;

    // Persisted before the dispatch, exactly as in shared mode: the recorded
    // address is what input and captures aim at, so it has to name the new
    // target even if the focus below fails.
    isolated.active_address = Some(window.address.clone());
    session::save_isolated(path, session, isolated)?;
    focus_in_instance(&signature, &window.address)?;

    Ok(format!(
        "target active — window {} (`{}`) focused in agent desktop `{session}` (instance \
         {signature})",
        window.address, window.title
    ))
}

/// §6.1: an agent desktop is destroyed whole, so a window in it has no
/// disposition to choose between.
fn refuse_disposition(on_teardown: Option<session::Disposition>) -> Result<(), Error> {
    let Some(disposition) = on_teardown else {
        return Ok(());
    };
    Err(Error::Invalid {
        what: "target option",
        value: format!("--on-teardown {}", disposition.label()),
        hint: "`teardown` takes the whole agent desktop down, so its windows have nothing to be \
               restored to or closed for; omit --on-teardown"
            .to_owned(),
    })
}

/// `--untracked` filters against the list of windows a shared session adopted;
/// an agent desktop records one window at a time instead, so the flag is
/// refused rather than quietly matching everything.
fn refuse_untracked(untracked: bool) -> Result<(), Error> {
    if !untracked {
        return Ok(());
    }
    Err(Error::Invalid {
        what: "target option",
        value: "--untracked".to_owned(),
        hint: "an agent desktop records one window at a time, not a list of adopted ones; select \
               the window with --match-title, --match-class, --pid or --address"
            .to_owned(),
    })
}

/// No match is a retry while `--wait` lasts, several are refused with the
/// candidates as JSON — the same contract as on the host, against the clients of
/// the instance.
fn instance_match<'a>(
    clients: &'a [hypr::Client],
    criteria: &session::Criteria<'_>,
) -> Result<Option<&'a hypr::Client>, Error> {
    match session::resolve(clients, criteria) {
        session::Resolution::Unique(client) => Ok(Some(client)),
        session::Resolution::None => Ok(None),
        session::Resolution::Ambiguous(candidates) => {
            Err(session::ambiguous_error(criteria, candidates))
        }
    }
}

fn wait_for_instance_window(
    signature: &str,
    session: &str,
    criteria: &session::Criteria<'_>,
    wait: Option<Duration>,
) -> Result<hypr::Client, Error> {
    let label = || {
        format!(
            "{} in agent desktop `{session}`",
            session::criteria_label(criteria)
        )
    };
    let started = Instant::now();
    loop {
        let clients = hypr::clients_on(Ctl::Instance(signature))?;
        if let Some(window) = instance_match(&clients, criteria)? {
            return Ok(window.clone());
        }
        let Some(timeout) = wait else {
            return Err(Error::WindowNotFound(label()));
        };
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            return Err(Error::Timeout {
                what: format!("a window matching {}", label()),
                after_ms: timeout.as_millis(),
            });
        }
        thread::sleep(session::POLL_INTERVAL.min(timeout.saturating_sub(elapsed)));
    }
}

/// The focus is read back from the instance: the dispatcher's `ok` only says the
/// request was accepted.
fn focus_in_instance(signature: &str, address: &str) -> Result<(), Error> {
    let ctl = Ctl::Instance(signature);
    hypr::dispatch_on(ctl, &["focuswindow", &format!("address:{address}")])?;
    poll_until(
        session::WINDOW_PLACE_TIMEOUT,
        || {
            Ok(match hypr::active_window_on(ctl)? {
                Some(window) if window.address == address => Ok(()),
                Some(window) => Err(format!(
                    "{} (`{}`) is focused",
                    window.address, window.title
                )),
                None => Err("no window is focused".to_owned()),
            })
        },
        |observed| {
            format!(
                "window {address} to become the focused window of agent desktop instance \
                 {signature} (last observed: {observed})"
            )
        },
    )
}

/// What `Isolated::shown` records. Both commands decide from where the console
/// actually is and write the flag afterwards; the flag is only the state's memory
/// of it, and a state that drifted must not answer for reality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    Shown,
    Hidden,
}

fn visibility(console_workspace: &str, agent_workspace: &str) -> Visibility {
    if console_workspace == agent_workspace {
        Visibility::Hidden
    } else {
        Visibility::Shown
    }
}

/// §5: the console window of the agent desktop goes to the workspace the user is
/// on, floating. It is the only host window this crate ever puts in front of the
/// user; it is identified by the address recorded at spawn, nothing else on the
/// desktop is touched, and the focus is left where the user had it — the window
/// is focusable (`noinitialfocus` was a one-shot rule, applied when it mapped),
/// so clicking it is theirs to decide. Rendering survives the move: a visible
/// window keeps receiving frame callbacks (fact §2.2), which is what lets
/// `capture_target` skip its headless-output check while `shown`.
pub fn show(session: &str, path: &Path, isolated: &mut Isolated) -> Result<String, Error> {
    let console_address = live_instance(session, isolated)?.console.to_owned();
    let console = host_console(&console_address, session)?;
    let focused = hypr::focused_workspace()?;
    if shown_where_the_user_looks(&console.workspace.name, &focused, &isolated.output) {
        return persist_visibility(
            session,
            path,
            isolated,
            Visibility::Shown,
            format!(
                "agent desktop `{session}` is already shown — console window {console_address} \
                 sits on workspace {}, the one in front of you",
                console.workspace.name
            ),
        );
    }
    let destination = user_workspace(&focused, &hypr::monitors()?, session, isolated)?;

    // Measured on the first live run of §5: `setfloating` does NOT drop the
    // fullscreen state the start's one-shot rule set, and Hyprland then refuses
    // `resizewindowpixel` with "Window is fullscreen". `fullscreenstate` takes no
    // window selector, so the console is focused for the length of the clear and
    // the user's focus and cursor are put back by the same envelope `--focus`
    // uses — a fullscreen console left as it is would cover their whole monitor.
    // The size is pinned to what the console had, because the agent desktop
    // renders at the size of this window: letting Hyprland pick a floating size
    // would silently change the resolution the agent has been working in.
    let size = console.size;
    guard::run(
        Some(&console_address),
        || Ok(()),
        |()| {
            hypr::dispatch(&["fullscreenstate", "0", "0"])?;
            hypr::dispatch(&["setfloating", &window_arg(&console_address)])?;
            resize_console(&console_address, size)?;
            hypr::dispatch(&[
                "movetoworkspacesilent",
                &format!(
                    "{},address:{console_address}",
                    session::workspace_selector(&destination)
                ),
            ])
        },
        |(), cursor| guard::restore_cursor(cursor),
    )?;

    let console = settle_console(
        &console_address,
        &ConsoleWant {
            workspace: &destination,
            floating: Some(true),
            at: None,
        },
    )?;
    warn_on_console_size(session, &console, size);
    persist_visibility(
        session,
        path,
        isolated,
        Visibility::Shown,
        format!(
            "agent desktop `{session}` shown — console window {console_address} is floating on \
             workspace {destination} ({}x{}); `hyprpilot --session {session} session hide` puts it \
             back on {}",
            console.size[0], console.size[1], isolated.workspace
        ),
    )
}

/// §5: the console goes back to `agent-<name>`, at the configured size and at
/// the origin of the headless output. Both matter: the agent desktop renders at
/// the size of this window, and a window Hyprland does not composite stops the
/// frame callbacks the whole design rests on (fact §2.2).
pub fn hide(session: &str, path: &Path, isolated: &mut Isolated) -> Result<String, Error> {
    let console_address = live_instance(session, isolated)?.console.to_owned();
    let console = host_console(&console_address, session)?;
    let already_hidden =
        visibility(&console.workspace.name, &isolated.workspace) == Visibility::Hidden;
    let size = window_size(isolated.size)?;

    // Geometry is only imposed on a console that actually moved; a hide of an
    // already hidden desktop touches nothing and only re-checks the invariant.
    let origin = if already_hidden {
        None
    } else {
        let origin = headless_origin(session, &isolated.output)?;
        hypr::dispatch(&[
            "movetoworkspacesilent",
            &format!(
                "{},address:{console_address}",
                session::workspace_selector(&isolated.workspace)
            ),
        ])?;
        resize_console(&console_address, size)?;
        hypr::dispatch(&[
            "movewindowpixel",
            &format!(
                "exact {} {},address:{console_address}",
                origin[0], origin[1]
            ),
        ])?;
        Some(origin)
    };

    let console = settle_console(
        &console_address,
        &ConsoleWant {
            workspace: &isolated.workspace,
            floating: None,
            at: origin,
        },
    )?;
    // Checked, not assumed: a console back on its workspace while that workspace
    // is no longer the active one on the headless output freezes every later
    // capture (fact §2.2).
    ensure_agent_frames(session, isolated)?;
    warn_on_console_size(session, &console, size);

    let state = if already_hidden {
        "is already hidden"
    } else {
        "hidden"
    };
    persist_visibility(
        session,
        path,
        isolated,
        Visibility::Hidden,
        format!(
            "agent desktop `{session}` {state} — console window {console_address} sits on \
             workspace {}, active on output {}",
            isolated.workspace, isolated.output
        ),
    )
}

/// The state's `shown` is written from where the console actually is, so it can
/// only disagree with reality between two commands.
fn persist_visibility(
    session: &str,
    path: &Path,
    isolated: &mut Isolated,
    visibility: Visibility,
    message: String,
) -> Result<String, Error> {
    let shown = visibility == Visibility::Shown;
    if isolated.shown != shown {
        isolated.shown = shown;
        session::save_isolated(path, session, isolated)?;
    }
    Ok(message)
}

/// The console as the host sees it. Identity is the address recorded at spawn:
/// nothing here matches on a class or a title, which change under us.
fn host_console(address: &str, session: &str) -> Result<hypr::Client, Error> {
    find_client(&hypr::clients()?, address)
        .cloned()
        .ok_or_else(|| Error::AgentDesktopUnready {
            session: session.to_owned(),
            reason: format!(
                "its console window {address} is gone from the host, so its nested compositor is \
                 no longer mapped anywhere — run `hyprpilot --session {session} teardown`"
            ),
        })
}

/// `show` is idempotent only while the console sits on the workspace the user is
/// actually looking at. `shown` on a workspace they have since switched away from
/// means an occluded console, so it is a desktop to move, not one to report as
/// already visible: an occluded console stops receiving frames (fact §2.2).
fn shown_where_the_user_looks(
    console_workspace: &str,
    focused: &hypr::FocusedWorkspace,
    agent_output: &str,
) -> bool {
    console_workspace == focused.name && focused.monitor != agent_output
}

/// The workspace `session show` moves the console to: the one the user is
/// looking at. A waybar click focuses the agent's own headless output (§7), and
/// moving the console onto the workspace it came from is not what was asked, so
/// that focus is refused instead.
fn user_workspace(
    focused: &hypr::FocusedWorkspace,
    monitors: &[hypr::Monitor],
    session: &str,
    isolated: &Isolated,
) -> Result<String, Error> {
    if focused.monitor == isolated.output || focused.name == isolated.workspace {
        return Err(Error::Invalid {
            what: "user workspace",
            value: format!("{} on {}", focused.name, focused.monitor),
            hint: format!(
                "the focus is on the agent desktop's own headless output {} (clicking its \
                 workspace in waybar does that); focus one of your own monitors, then run \
                 `hyprpilot --session {session} session show` again",
                isolated.output
            ),
        });
    }
    let monitor = monitors
        .iter()
        .find(|monitor| monitor.name == focused.monitor)
        .ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!(
                "`hyprctl activeworkspace` reports workspace {} on {}, which `hyprctl monitors` \
                 does not list",
                focused.name, focused.monitor
            ),
        })?;
    // `activeworkspace` answers with the workspace *underneath* an open special
    // workspace, so the console would arrive already occluded — which freezes the
    // nested compositor and blocks every later capture (fact §2.2).
    if !monitor.special_workspace.is_empty() {
        return Err(Error::Invalid {
            what: "user workspace",
            value: format!("{} on {}", focused.name, focused.monitor),
            hint: format!(
                "special workspace {} covers {}, so the console would land under it and stop \
                 receiving frames; close it, then run `hyprpilot --session {session} session show` \
                 again",
                monitor.special_workspace, focused.monitor
            ),
        });
    }
    Ok(focused.name.clone())
}

fn window_arg(address: &str) -> String {
    format!("address:{address}")
}

/// The configured size as window geometry. `hyprctl clients` reports sizes as
/// signed pixels, so a configured size that cannot be one is refused instead of
/// silently clamped.
fn window_size(size: [u32; 2]) -> Result<[i32; 2], Error> {
    let pixel = |value: u32| {
        i32::try_from(value).map_err(|_| Error::Invalid {
            what: "agent desktop size",
            value: format!("{}x{}", size[0], size[1]),
            hint: "each dimension must fit a signed 32-bit pixel count".to_owned(),
        })
    };
    Ok([pixel(size[0])?, pixel(size[1])?])
}

fn resize_console(address: &str, size: [i32; 2]) -> Result<(), Error> {
    hypr::dispatch(&[
        "resizewindowpixel",
        &format!("exact {} {},address:{address}", size[0], size[1]),
    ])
}

/// The agent desktop renders at the size of its console window, so a size the
/// move could not preserve changes the coordinate space the agent works in. It
/// is reported rather than fought over: the console is where it was asked to be,
/// and `status` shows the effective size read inside the instance.
fn warn_on_console_size(session: &str, console: &hypr::Client, expected: [i32; 2]) {
    if console.size == expected {
        return;
    }
    let _ = writeln!(
        std::io::stderr(),
        "hyprpilot: warning: console window {} of agent desktop `{session}` is {}x{} instead of \
         {}x{} — the agent desktop now renders at that size; check `hyprpilot --session {session} \
         status`",
        console.address,
        console.size[0],
        console.size[1],
        expected[0],
        expected[1],
    );
}

/// Where the console has to land to be composited again: the origin of its
/// headless output. A layout coordinate that is not integral is rounded rather
/// than refused — `movewindowpixel` speaks pixels, unlike the exact conversion
/// the shared parking path needs.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the rounded value is range-checked against i32 before the cast"
)]
fn headless_origin(session: &str, output: &str) -> Result<[i32; 2], Error> {
    let monitor = hypr::monitors()?
        .into_iter()
        .find(|monitor| monitor.name == output)
        .ok_or_else(|| Error::AgentDesktopUnready {
            session: session.to_owned(),
            reason: format!(
                "its headless output {output} is gone — run `hyprpilot --session {session} \
                 teardown`"
            ),
        })?;
    let pixel = |value: f64, field: &str| {
        let rounded = value.round();
        if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
            return Err(Error::Tool {
                command: "hyprctl monitors".to_owned(),
                message: format!("output {output} reports unusable {field} {value}"),
            });
        }
        Ok(rounded as i32)
    };
    Ok([pixel(monitor.x, "x")?, pixel(monitor.y, "y")?])
}

/// What a console move has to end up as. `None` means "not asked for", so a move
/// is never verified against something it did not request.
struct ConsoleWant<'a> {
    workspace: &'a str,
    floating: Option<bool>,
    at: Option<[i32; 2]>,
}

fn console_settled(console: &hypr::Client, want: &ConsoleWant<'_>) -> Result<(), String> {
    if console.workspace.name != want.workspace {
        return Err(format!("on workspace {}", console.workspace.name));
    }
    if want.floating == Some(!console.floating) {
        return Err(format!("floating is {}", console.floating));
    }
    if let Some(at) = want.at
        && console.at != at
    {
        return Err(format!("at {:?}", console.at));
    }
    Ok(())
}

/// Bounded read-back of the console after a move: the dispatcher's `ok` only
/// says the request was accepted.
fn settle_console(address: &str, want: &ConsoleWant<'_>) -> Result<hypr::Client, Error> {
    poll_until(
        session::WINDOW_PLACE_TIMEOUT,
        || {
            let Some(console) = find_client(&hypr::clients()?, address).cloned() else {
                return Err(Error::WindowGone(address.to_owned()));
            };
            Ok(console_settled(&console, want).map(|()| console))
        },
        |observed| {
            format!(
                "console window {address} to sit on workspace {} (last observed: {observed})",
                want.workspace
            )
        },
    )
}

/// Fact §2.2, checked at the end of `hide` rather than assumed: the nested
/// compositor only keeps receiving frame callbacks while its console is
/// composited, which is what an active agent workspace on the headless output
/// guarantees.
fn ensure_agent_frames(session: &str, isolated: &Isolated) -> Result<(), Error> {
    poll_until(
        session::WINDOW_PLACE_TIMEOUT,
        || {
            Ok(host_frames(
                &hypr::monitors()?,
                &isolated.output,
                &isolated.workspace,
            ))
        },
        |observed| {
            format!(
                "workspace {} to be the active one again on output {} — {}",
                isolated.workspace,
                isolated.output,
                frozen_reason(session, &isolated.output, &isolated.workspace, observed)
            )
        },
    )
}

/// Identity of a process of an agent desktop: the session marker every process of
/// the desktop inherits, **and** the nonce of the start that spawned it. The
/// session marker alone is inheritable — a shell that exported it, or anything
/// launched inside an agent desktop, carries it — so it is never enough to
/// select a process to signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Marker<'a> {
    session: &'a str,
    instance: &'a str,
}

impl Marker<'_> {
    fn label(&self) -> String {
        format!(
            "{AGENT_SESSION_ENV}={} and {AGENT_INSTANCE_ENV}={}",
            self.session, self.instance
        )
    }
}

/// Every process of an agent desktop inherits both markers, including the app the
/// nested compositor launched, so the environment identifies the whole desktop.
/// A process is never identified by its binary name — and never by one marker
/// alone. This process itself is excluded: a command run from inside an agent
/// desktop is refused long before this, but a sweep must not be able to signal
/// its own pid whatever put the markers in its environment.
fn marked_pids_in(proc_root: &Path, marker: &Marker<'_>) -> Result<Vec<u32>, Error> {
    let context = || format!("listing {}", proc_root.display());
    let entries = fs::read_dir(proc_root).map_err(|source| Error::Io {
        context: context(),
        source,
    })?;
    let own = std::process::id();
    let mut pids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            context: context(),
            source,
        })?;
        let Some(pid) = parse_pid(&entry.file_name()) else {
            continue;
        };
        if pid != own && environ_carries_marker(&entry.path(), marker) {
            pids.push(pid);
        }
    }
    pids.sort_unstable();
    Ok(pids)
}

fn parse_pid(name: &OsStr) -> Option<u32> {
    name.to_str()?.parse().ok()
}

/// A process that exits mid-scan, or whose environment we may not read, is not
/// one of ours to report.
fn environ_carries_marker(proc_dir: &Path, marker: &Marker<'_>) -> bool {
    let session = format!("{AGENT_SESSION_ENV}={}", marker.session);
    let instance = format!("{AGENT_INSTANCE_ENV}={}", marker.instance);
    fs::read(proc_dir.join("environ")).is_ok_and(|environ| {
        let mut carries_session = false;
        let mut carries_instance = false;
        for variable in environ.split(|byte| *byte == 0) {
            carries_session |= variable == session.as_bytes();
            carries_instance |= variable == instance.as_bytes();
        }
        carries_session && carries_instance
    })
}

fn process_carries_marker(pid: i32, marker: &Marker<'_>) -> Result<bool, Error> {
    let pid = u32::try_from(pid).map_err(|_| Error::Tool {
        command: "hyprctl clients".to_owned(),
        message: format!("client reports invalid pid {pid}"),
    })?;
    Ok(pid_carries_marker(pid, marker))
}

/// Liveness *and* identity of a recorded pid: a dead pid has no readable
/// environment, and a recycled one carries neither marker of this start.
fn pid_carries_marker(pid: u32, marker: &Marker<'_>) -> bool {
    pid_carries_marker_in(Path::new("/proc"), pid, marker)
}

fn pid_carries_marker_in(proc_root: &Path, pid: u32, marker: &Marker<'_>) -> bool {
    environ_carries_marker(&proc_root.join(pid.to_string()), marker)
}

/// The effects of the desktop-wide sweep, injected so the ladder is testable
/// against a fake `/proc` and without signalling anything.
struct Sweep<'a> {
    proc_root: &'a Path,
    marker: &'a Marker<'a>,
    signal: &'a dyn Fn(u32, &'static str) -> Result<(), Error>,
    ladder: session::Escalation,
}

/// Nothing of this desktop may outlive a teardown or a rolled-back start, and
/// only the marker pair says what belongs to it. The ladder is §6.2's: the polite
/// `dispatch exit` its caller already sent gets its grace period first — that is
/// what lets the nested compositor remove its own runtime directory (fact §2.9) —
/// then `SIGTERM`, then `SIGKILL`, then a refusal.
fn terminate_marked(marker: &Marker<'_>) -> Result<Vec<String>, RestoreFailure> {
    terminate_marked_in(&Sweep {
        proc_root: Path::new("/proc"),
        marker,
        signal: &session::signal_process,
        ladder: EXIT_ESCALATION,
    })
}

fn terminate_marked_in(sweep: &Sweep<'_>) -> Result<Vec<String>, RestoreFailure> {
    let started = Instant::now();
    let failure = |actual: String| RestoreFailure {
        what: "agent desktop",
        expected: format!("no process left carrying {}", sweep.marker.label()),
        actual,
    };
    let mut notes = Vec::new();
    let mut step = None;
    // Each pid is signalled once per rung, not once per poll — and a process that
    // only appears after the first signal still gets one.
    let mut signalled = BTreeSet::new();
    loop {
        let pids = marked_pids_in(sweep.proc_root, sweep.marker).map_err(|error| {
            failure(format!(
                "scanning {} failed: {error}",
                sweep.proc_root.display()
            ))
        })?;
        if pids.is_empty() {
            return Ok(notes);
        }
        match sweep.ladder.step(started.elapsed()) {
            session::Step::Wait => {}
            session::Step::Signal(signal) => {
                if step != Some(signal) {
                    step = Some(signal);
                    signalled.clear();
                }
                let fresh = pids
                    .iter()
                    .copied()
                    .filter(|pid| !signalled.contains(pid))
                    .collect::<Vec<_>>();
                if !fresh.is_empty() {
                    notes.push(format!(
                        "SIG{signal} sent to pids {fresh:?} of the agent desktop"
                    ));
                }
                for pid in fresh {
                    let _ = (sweep.signal)(pid, signal);
                    signalled.insert(pid);
                }
            }
            session::Step::GiveUp => return Err(failure(format!("pids {pids:?} still alive"))),
        }
        thread::sleep(sweep.ladder.poll);
    }
}

/// Bounded settle loop: never a blind sleep, and a timeout reports the last
/// observation.
fn poll_until<T>(
    timeout: Duration,
    mut probe: impl FnMut() -> Probe<T>,
    what: impl FnOnce(&str) -> String,
) -> Result<T, Error> {
    let deadline = Instant::now() + timeout;
    loop {
        let observed = match probe()? {
            Ok(value) => return Ok(value),
            Err(observed) => observed,
        };
        if Instant::now() >= deadline {
            return Err(Error::Timeout {
                what: what(&observed),
                after_ms: timeout.as_millis(),
            });
        }
        thread::sleep(session::POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::error::Error as StdError;
    use std::fs;
    use std::path::Path;

    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use super::{
        AGENT_SESSION_ENV, Compositor, Console, ConsoleWant, Exit, FrameSite, HostSnapshot, Keymap,
        LiveInstance, Marker, Registered, Sweep, TeardownPlan, Visibility, active_workspaces,
        capturable, capture_target, clear_runtime_dir, clear_stale_socket, console_reaped,
        console_settled, deviation, ensure_output_absent, frame_site, frames_reason, frozen_reason,
        instance_dir, instance_match, instance_nonce, instance_signature, keep_instance_log,
        keymap_of, live_instance_in, marked_instance, marked_pids_in, nested_config,
        output_is_configured, output_vacated, persist_visibility, plain_signature, recorded_window,
        refuse_disposition, refuse_nested_marker, refuse_untracked, remove_instance_dir,
        renameable, select_console, shell_path, shown_where_the_user_looks, spawn_command,
        stop_instance, teardown_plan, terminate_marked_in, unwind, user_workspace, visibility,
        workspace_occupants,
    };
    use crate::error::Error;
    use crate::host::ledger::{self, HostMutation};
    use crate::hypr::{Client, Devices, FocusedWorkspace, Monitor};
    use crate::session::{self, Escalation, Instance, Isolated};

    const MONITORS_JSON: &str = include_str!("../fixtures/monitors.json");
    /// What a nested compositor reports: exactly one output (§4.4 pins a single
    /// `monitor = ,…` rule), which is the layout every isolated capture and every
    /// pointer warp works in.
    const NESTED_MONITORS_JSON: &str = include_str!("../fixtures/monitors-nested.json");
    const DEVICES_JSON: &str = include_str!("../fixtures/devices.json");
    const NESTED_CLIENTS_JSON: &str = include_str!("../fixtures/clients-nested.json");
    const INSTANCE_CLIENTS_JSON: &str = include_str!("../fixtures/clients-ambiguous.json");
    /// A nonce as a start generates one: pid plus nanoseconds.
    const NONCE: &str = "4242-1700000000000000000";

    fn monitors() -> Result<Vec<Monitor>, serde_json::Error> {
        serde_json::from_str(MONITORS_JSON)
    }

    fn nested_monitors() -> Result<Vec<Monitor>, serde_json::Error> {
        serde_json::from_str(NESTED_MONITORS_JSON)
    }

    fn instance_clients() -> Result<Vec<Client>, serde_json::Error> {
        serde_json::from_str(INSTANCE_CLIENTS_JSON)
    }

    fn sample_keymap() -> Keymap {
        Keymap {
            rules: String::new(),
            model: "pc105".to_owned(),
            layout: "fr".to_owned(),
            variant: "azerty".to_owned(),
            options: "compose:caps".to_owned(),
        }
    }

    /// A host with `outputs` showing those workspaces, and `also` as workspaces
    /// that exist without being visible anywhere — the empty scratch pad a user
    /// keeps is one of those.
    fn snapshot_with(outputs: &[(&str, &str)], also: &[&str], focus: Option<&str>) -> HostSnapshot {
        HostSnapshot {
            workspaces: outputs
                .iter()
                .map(|(output, workspace)| ((*output).to_owned(), (*workspace).to_owned()))
                .collect(),
            workspace_names: outputs
                .iter()
                .map(|(_, workspace)| (*workspace).to_owned())
                .chain(also.iter().map(|name| (*name).to_owned()))
                .collect(),
            active_window: focus.map(str::to_owned),
            cursor: (100, 200),
        }
    }

    fn snapshot(outputs: &[(&str, &str)], focus: Option<&str>) -> HostSnapshot {
        snapshot_with(outputs, &[], focus)
    }

    fn marker() -> Marker<'static> {
        Marker {
            session: "alpha",
            instance: NONCE,
        }
    }

    /// Writes a `/proc/<pid>/environ` blob under `root`.
    fn fake_environ(root: &Path, pid: &str, variables: &[&str]) -> Result<(), Box<dyn StdError>> {
        let dir = root.join(pid);
        fs::create_dir_all(&dir)?;
        let mut blob = Vec::new();
        for variable in variables {
            blob.extend_from_slice(variable.as_bytes());
            blob.push(0);
        }
        fs::write(dir.join("environ"), blob)?;
        Ok(())
    }

    #[test]
    fn nested_config_pins_the_layout_and_inherits_the_host_keymap() {
        let config = nested_config("alpha", [1600, 1000], &sample_keymap());
        let lines = config.lines().collect::<Vec<_>>();

        for expected in [
            "monitor = , 1600x1000@60, 0x0, 1",
            "general {",
            "    gaps_in = 0",
            "    gaps_out = 0",
            "    border_size = 0",
            "    rounding = 0",
            "animations {",
            "    force_default_wallpaper = 0",
            "    background_color = rgb(1e1e2e)",
            "    disable_watchdog_warning = true",
            "input {",
            "    kb_rules = ",
            "    kb_model = pc105",
            "    kb_layout = fr",
            "    kb_variant = azerty",
            "    kb_options = compose:caps",
            "    disable_logs = false",
        ] {
            assert!(lines.contains(&expected), "missing line `{expected}`");
        }
        // `animations { enabled = false }` and the two decoration effects.
        assert_eq!(
            lines
                .iter()
                .filter(|line| *line == &"    enabled = false")
                .count(),
            1
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| *line == &"        enabled = false")
                .count(),
            2
        );
        assert!(config.contains("agent desktop `alpha`"), "{config}");
        assert!(config.contains("hyprpilot-alpha"), "{config}");
        assert!(!config.contains("exec-once"), "{config}");
        assert!(!config.to_lowercase().contains("portal"), "{config}");
    }

    #[test]
    fn keymap_comes_from_the_host_or_the_start_fails() -> Result<(), Box<dyn StdError>> {
        let devices: Devices = serde_json::from_str(DEVICES_JSON)?;
        let keymap = keymap_of(&devices)?;
        assert_eq!(
            keymap,
            Keymap {
                rules: String::new(),
                model: String::new(),
                layout: "us".to_owned(),
                variant: String::new(),
                options: "compose:caps".to_owned(),
            }
        );

        let mut orphan = devices;
        for keyboard in &mut orphan.keyboards {
            keyboard.main = false;
        }
        let error = keymap_of(&orphan)
            .err()
            .ok_or("a keyboardless host was accepted")?
            .to_string();
        assert!(error.contains("no main keyboard"), "{error}");
        assert!(error.contains("hyprctl devices"), "{error}");
        Ok(())
    }

    fn instance_info(instance: &str, pid: i32, socket: &str) -> crate::hypr::InstanceInfo {
        crate::hypr::InstanceInfo {
            instance: instance.to_owned(),
            pid,
            wl_socket: socket.to_owned(),
        }
    }

    #[test]
    fn an_instance_is_attributed_by_its_markers_not_by_what_appeared()
    -> Result<(), Box<dyn StdError>> {
        let instances = vec![
            instance_info("cafe_1700000000", 10, "wayland-1"),
            instance_info("beef_1700000001", 20, "wayland-2"),
            instance_info("dead_1700000002", 30, "wayland-3"),
        ];

        // The user's own compositor and a concurrent start's are both present, and
        // neither is a candidate: the socket comes from the same record, so it can
        // never be the one a racing start opened.
        let found = marked_instance(&instances, &|pid: i32| Ok(pid == 20))?
            .ok_or("the marked instance was not found")?;
        assert_eq!(found.instance, "beef_1700000001");
        assert_eq!(found.wl_socket, "wayland-2");

        assert!(marked_instance(&instances, &|_| Ok(false))?.is_none());

        let Err(error) = marked_instance(&instances, &|pid: i32| Ok(pid != 10)) else {
            return Err("two marked instances must never be guessed between".into());
        };
        assert!(error.to_string().contains("two live instances"), "{error}");
        Ok(())
    }

    #[test]
    fn a_stale_socket_is_unlinked_but_a_live_one_is_never_touched() -> Result<(), Box<dyn StdError>>
    {
        let dir = tempfile::tempdir()?;

        // Nothing recorded left behind: an idempotent success.
        let (notes, failure) = clear_stale_socket(dir.path(), "wayland-9");
        assert!(failure.is_none());
        assert_eq!(notes, vec!["socket wayland-9 already gone".to_owned()]);

        // A dead compositor's leftovers: socket and lock both go (libwayland
        // only unlinks them on a clean exit).
        fs::write(dir.path().join("wayland-7"), b"")?;
        fs::write(dir.path().join("wayland-7.lock"), b"")?;
        let (notes, failure) = clear_stale_socket(dir.path(), "wayland-7");
        assert!(failure.is_none(), "{failure:?}");
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(!dir.path().join("wayland-7").exists());
        assert!(!dir.path().join("wayland-7.lock").exists());

        // A socket someone still accepts on is refused: the recorded name could
        // have been taken over by another compositor after the kill.
        let live = dir.path().join("wayland-8");
        let listener = std::os::unix::net::UnixListener::bind(&live)?;
        let (notes, failure) = clear_stale_socket(dir.path(), "wayland-8");
        assert!(notes.is_empty(), "{notes:?}");
        let failure = failure.ok_or("a live socket was unlinked")?;
        assert!(failure.actual.contains("still accepts"), "{failure:?}");
        assert!(live.exists(), "a live socket must survive");
        drop(listener);
        Ok(())
    }

    #[test]
    fn console_is_selected_by_class_and_marker_never_by_title() -> Result<(), Box<dyn StdError>> {
        let clients: Vec<Client> = serde_json::from_str(NESTED_CLIENTS_JSON)?;
        let ours = |pid: i32| Ok(pid == 4242);
        let nothing = BTreeSet::new();

        // `0xdecoy` is an `aquamarine` window of another pid and `0xstale`
        // shares our pid with another class: neither is the console.
        let console = select_console(&clients, &nothing, &ours)?.ok_or("console not selected")?;
        assert_eq!(console.address, "0xc0ff33");
        assert_eq!(console.pid, 4242);

        // Hyprland addresses are recycled pointers: a console that inherited the
        // address of a window closed since the snapshot is still ours, and
        // discarding it would burn the whole appear timeout for nothing.
        let known = clients
            .iter()
            .map(|client| client.address.clone())
            .collect::<BTreeSet<_>>();
        let recycled = select_console(&clients, &known, &ours)?
            .ok_or("a console at a recycled address was discarded")?;
        assert_eq!(recycled.address, "0xc0ff33");

        let anything = |_: i32| Ok(true);
        let error = select_console(&clients, &nothing, &anything)
            .err()
            .ok_or("two marked consoles were accepted")?
            .to_string();
        assert!(error.contains("two console windows"), "{error}");
        assert!(error.contains("0xc0ff33"), "{error}");
        assert!(error.contains("0xdecoy"), "{error}");
        // The snapshot is left to enrich an ambiguity, which is all it can honestly
        // say about an address.
        let annotated = select_console(&clients, &known, &anything)
            .err()
            .ok_or("two marked consoles were accepted")?
            .to_string();
        assert!(annotated.contains("predates this start"), "{annotated}");

        let unreadable = |_: i32| {
            Err(Error::Tool {
                command: "read /proc".to_owned(),
                message: "denied".to_owned(),
            })
        };
        assert!(
            select_console(&clients, &nothing, &unreadable).is_err(),
            "a failed marker read must abort, not skip the window"
        );
        Ok(())
    }

    #[test]
    fn the_nested_marker_refuses_every_command_not_only_a_start() -> Result<(), Box<dyn StdError>> {
        refuse_nested_marker(None)?;

        for marker in ["alpha", ""] {
            let error = refuse_nested_marker(Some(marker))
                .err()
                .ok_or("a command inside an agent desktop was accepted")?;
            assert!(matches!(&error, Error::NestedRefused { .. }));
            let message = error.to_string();
            assert!(message.contains(AGENT_SESSION_ENV), "{message}");
            // A start would build a 0x0 output (fact §2.7); a shared start would
            // do the same and report success (§2.7 again); and anything sweeping by
            // marker would reach the caller's own shell.
            assert!(message.contains("0x0"), "{message}");
            assert!(
                message.contains("shell this command was typed in"),
                "{message}"
            );
        }
        Ok(())
    }

    #[test]
    fn existing_agent_output_is_a_leftover_not_a_resource() -> Result<(), Box<dyn StdError>> {
        let monitors = monitors()?;
        ensure_output_absent(&monitors, "hyprpilot-alpha", "alpha")?;

        let error = ensure_output_absent(&monitors, "headless-ci", "alpha")
            .err()
            .ok_or("a pre-existing output was reused")?;
        assert!(matches!(&error, Error::AgentOutputExists { .. }));
        let message = error.to_string();
        assert!(
            message.contains("hyprpilot --session alpha teardown"),
            "{message}"
        );
        assert!(
            message.contains("hyprctl output remove headless-ci"),
            "{message}"
        );
        Ok(())
    }

    #[test]
    fn output_check_requires_the_requested_mode_and_scale_one() -> Result<(), Box<dyn StdError>> {
        let mut monitor = monitors()?.swap_remove(1);
        assert!(output_is_configured(&monitor, [1600, 1000]));
        assert!(!output_is_configured(&monitor, [1920, 1080]));

        // Fact §2.10: an inherited scale must fail the check.
        monitor.scale = 2.0;
        assert!(!output_is_configured(&monitor, [1600, 1000]));
        Ok(())
    }

    #[test]
    fn host_snapshot_tolerates_the_new_output_and_nothing_else() -> Result<(), Box<dyn StdError>> {
        assert_eq!(
            active_workspaces(&monitors()?),
            [("DP-3", "1"), ("headless-ci", "proto")]
                .into_iter()
                .map(|(output, workspace)| (output.to_owned(), workspace.to_owned()))
                .collect::<BTreeMap<_, _>>()
        );

        let before = snapshot(&[("DP-3", "1")], Some("0xabc"));
        assert!(deviation(&before, &snapshot(&[("DP-3", "1")], Some("0xabc"))).is_none());
        assert!(
            deviation(
                &before,
                &snapshot(
                    &[("DP-3", "1"), ("hyprpilot-alpha", "agent-alpha")],
                    Some("0xabc")
                )
            )
            .is_none(),
            "the output this start created is not a deviation"
        );

        let workspace = deviation(&before, &snapshot(&[("DP-3", "2")], Some("0xabc")))
            .ok_or("a workspace switch went unnoticed")?
            .to_string();
        assert!(workspace.contains("active workspace"), "{workspace}");
        assert!(workspace.contains("expected 1 on DP-3"), "{workspace}");
        assert!(workspace.contains("observed 2 on DP-3"), "{workspace}");
        assert!(workspace.contains("rolled back"), "{workspace}");

        let focus = deviation(&before, &snapshot(&[("DP-3", "1")], Some("0xdef")))
            .ok_or("a focus change went unnoticed")?
            .to_string();
        assert!(focus.contains("address:0xabc"), "{focus}");
        assert!(focus.contains("address:0xdef"), "{focus}");
        assert!(
            deviation(&before, &snapshot(&[("DP-3", "1")], None))
                .is_some_and(|error| error.to_string().contains("no focused window")),
            "losing focus is a deviation too"
        );
        assert!(
            deviation(&before, &snapshot(&[], Some("0xabc"))).is_some(),
            "a host output that vanished is a deviation"
        );
        Ok(())
    }

    #[test]
    fn capturable_accepts_an_agent_workspace_active_with_a_mapped_window()
    -> Result<(), Box<dyn StdError>> {
        let monitors = monitors()?;
        // The agent desktop has exactly one output, so the instance side is read
        // from a single-output layout — the only one it can ever report (§4.4).
        let outputs = nested_monitors()?;
        let clients = instance_clients()?;

        // Host side: `headless-ci` shows `proto`. Instance side: `0xaaa` sits on
        // workspace `1`, which is what the one output is showing.
        capturable(
            &monitors,
            &outputs,
            &clients,
            "headless-ci",
            "proto",
            "0xaaa",
        )?;

        let frozen = capturable(
            &monitors,
            &outputs,
            &clients,
            "headless-ci",
            "agent-alpha",
            "0xaaa",
        )
        .err()
        .ok_or("an inactive agent workspace was accepted")?;
        assert!(frozen.contains("stop receiving frames"), "{frozen}");

        let absent = capturable(
            &monitors,
            &outputs,
            &clients,
            "hyprpilot-alpha",
            "agent-alpha",
            "0xaaa",
        )
        .err()
        .ok_or("a missing host output was accepted")?;
        assert!(absent.contains("hyprpilot-alpha is absent"), "{absent}");

        let mut occluded = monitors;
        occluded[1].special_workspace = "special:magic".to_owned();
        let occluded = capturable(
            &occluded,
            &outputs,
            &clients,
            "headless-ci",
            "proto",
            "0xaaa",
        )
        .err()
        .ok_or("an occluded headless output was accepted")?;
        assert!(occluded.contains("special:magic"), "{occluded}");
        Ok(())
    }

    #[test]
    fn capturable_refuses_a_window_that_is_gone_unmapped_or_hidden() -> Result<(), Box<dyn StdError>>
    {
        let monitors = monitors()?;
        let outputs = nested_monitors()?;
        let mut clients = instance_clients()?;

        let gone = capturable(
            &monitors,
            &outputs,
            &clients,
            "headless-ci",
            "proto",
            "0xzzz",
        )
        .err()
        .ok_or("an absent window was accepted")?;
        assert!(gone.contains("0xzzz is gone"), "{gone}");

        // `0xbbb` sits on workspace `2` while the agent desktop's one output
        // shows `1`.
        let hidden = capturable(
            &monitors,
            &outputs,
            &clients,
            "headless-ci",
            "proto",
            "0xbbb",
        )
        .err()
        .ok_or("a window on a hidden workspace was accepted")?;
        assert!(hidden.contains("while the agent desktop shows"), "{hidden}");

        clients[0].size = [0, 0];
        let unmapped = capturable(
            &monitors,
            &outputs,
            &clients,
            "headless-ci",
            "proto",
            "0xaaa",
        )
        .err()
        .ok_or("an unmapped window was accepted")?;
        assert!(unmapped.contains("not mapped"), "{unmapped}");

        clients[0].size = [800, 600];
        clients[0].monitor = 7;
        let stray = capturable(
            &monitors,
            &outputs,
            &clients,
            "headless-ci",
            "proto",
            "0xaaa",
        )
        .err()
        .ok_or("a window on an unknown monitor was accepted")?;
        assert!(stray.contains("monitor 7"), "{stray}");
        // The agent desktop has one output, so a second monitor id is never a
        // place a window of it can live.
        assert_eq!(
            outputs.len(),
            1,
            "the nested fixture must stay single-output"
        );
        Ok(())
    }

    #[test]
    fn spawn_command_carries_the_one_shot_rules_marker_and_log() -> Result<(), Box<dyn StdError>> {
        let command = spawn_command(
            &marker(),
            "agent-alpha",
            Path::new("/run/user/1000/hyprpilot/sessions/alpha/hyprland.conf"),
            Path::new("/run/user/1000/hyprpilot/sessions/alpha/hyprland.log"),
        )?;

        // Both markers: the session one names the desktop, the instance nonce is
        // what makes the pair an identity nothing else can inherit by accident.
        assert_eq!(
            command,
            "[workspace name:agent-alpha silent; noinitialfocus; fullscreen] env \
             HYPRPILOT_AGENT_SESSION=alpha \
             HYPRPILOT_AGENT_INSTANCE=4242-1700000000000000000 Hyprland -c \
             '/run/user/1000/hyprpilot/sessions/alpha/hyprland.conf' > \
             '/run/user/1000/hyprpilot/sessions/alpha/hyprland.log' 2>&1"
        );

        // The command reaches `sh -c`, so a path that could close the quote is
        // refused instead of escaped.
        assert!(shell_path(Path::new("/run/user/1000/ok")).is_ok());
        assert!(shell_path(Path::new("/run/user/1000/it's")).is_err());
        assert!(
            spawn_command(
                &marker(),
                "agent-alpha",
                Path::new("/run/user/1000/it's/hyprland.conf"),
                Path::new("/run/user/1000/hyprland.log"),
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn a_start_nonce_is_unique_and_shell_safe() -> Result<(), Box<dyn StdError>> {
        let first = instance_nonce()?;
        let second = instance_nonce()?;

        assert_ne!(first, second, "two starts must not share a nonce");
        assert!(
            first.starts_with(&format!("{}-", std::process::id())),
            "{first}"
        );
        // It ends up in a `sh -c` command line, so nothing in it may need quoting.
        assert!(
            first.chars().all(|c| c.is_ascii_digit() || c == '-'),
            "{first}"
        );
        Ok(())
    }

    #[test]
    fn marked_pids_cover_every_process_of_the_agent_desktop() -> Result<(), Box<dyn StdError>> {
        let root = tempfile::tempdir()?;
        let nonce = format!("HYPRPILOT_AGENT_INSTANCE={NONCE}");
        let fake_process = |pid: &str, environ: &[&str]| fake_environ(root.path(), pid, environ);

        fake_process(
            "11",
            &["PATH=/usr/bin", "HYPRPILOT_AGENT_SESSION=alpha", &nonce],
        )?;
        fake_process("12", &["HYPRPILOT_AGENT_SESSION=beta", &nonce])?;
        // The app the nested compositor launched inherits both markers.
        fake_process(
            "13",
            &[
                "HYPRPILOT_AGENT_SESSION=alpha",
                &nonce,
                "WAYLAND_DISPLAY=wayland-2",
            ],
        )?;
        // Another desktop whose name merely starts the same.
        fake_process("14", &["HYPRPILOT_AGENT_SESSION=alpha2", &nonce])?;
        // The incident this pair exists for: a shell of session `alpha` that
        // exported the session marker, or anything launched inside that desktop
        // by an *earlier* start. One marker is not an identity.
        fake_process("16", &["HYPRPILOT_AGENT_SESSION=alpha"])?;
        fake_process(
            "17",
            &[
                "HYPRPILOT_AGENT_SESSION=alpha",
                "HYPRPILOT_AGENT_INSTANCE=99-1",
            ],
        )?;
        // A process carrying only the nonce is no more ours than one carrying
        // only the session.
        fake_process("18", &[&nonce])?;
        fs::create_dir_all(root.path().join("15"))?;
        fs::create_dir_all(root.path().join("self"))?;

        assert_eq!(marked_pids_in(root.path(), &marker())?, vec![11, 13]);
        assert_eq!(
            marked_pids_in(
                root.path(),
                &Marker {
                    session: "alpha2",
                    instance: NONCE
                }
            )?,
            vec![14]
        );
        assert!(
            marked_pids_in(
                root.path(),
                &Marker {
                    session: "gamma",
                    instance: NONCE
                }
            )?
            .is_empty()
        );
        Ok(())
    }

    #[test]
    fn a_sweep_never_selects_its_own_process() -> Result<(), Box<dyn StdError>> {
        let root = tempfile::tempdir()?;
        let own = std::process::id();
        // Both markers, and still not a pid to signal: a `teardown` run from a
        // shell that carries them would otherwise kill the process running it.
        fake_environ(
            root.path(),
            &own.to_string(),
            &[
                "HYPRPILOT_AGENT_SESSION=alpha",
                &format!("HYPRPILOT_AGENT_INSTANCE={NONCE}"),
            ],
        )?;

        assert!(marked_pids_in(root.path(), &marker())?.is_empty());
        Ok(())
    }

    #[test]
    fn a_marked_desktop_dies_through_the_exit_ladder() -> Result<(), Box<dyn StdError>> {
        let root = tempfile::tempdir()?;
        let nonce = format!("HYPRPILOT_AGENT_INSTANCE={NONCE}");
        for pid in ["21", "22"] {
            fake_environ(root.path(), pid, &["HYPRPILOT_AGENT_SESSION=alpha", &nonce])?;
        }
        // Pid 21 goes down on SIGTERM, pid 22 only on SIGKILL.
        let signals: RefCell<Vec<(u32, &'static str)>> = RefCell::new(Vec::new());
        let reaper = |pid: u32, signal: &'static str| {
            signals.borrow_mut().push((pid, signal));
            if (pid == 21 && signal == "TERM") || signal == "KILL" {
                let _ = fs::remove_dir_all(root.path().join(pid.to_string()));
            }
            Ok(())
        };

        let notes = terminate_marked_in(&Sweep {
            proc_root: root.path(),
            marker: &marker(),
            signal: &reaper,
            ladder: TEST_LADDER,
        })
        .map_err(|failure| failure.actual)?;

        // The polite `dispatch exit` its caller already sent gets the grace period
        // first (fact §2.9: that is what lets the nested compositor remove its own
        // runtime directory), and only then SIGTERM, then SIGKILL.
        assert_eq!(
            *signals.borrow(),
            vec![(21, "TERM"), (22, "TERM"), (22, "KILL")],
            "{notes:?}"
        );
        assert!(
            notes.iter().any(|note| note.contains("SIGTERM sent")),
            "{notes:?}"
        );
        assert!(
            notes.iter().any(|note| note.contains("SIGKILL sent")),
            "{notes:?}"
        );
        Ok(())
    }

    #[test]
    fn a_desktop_that_survives_the_ladder_refuses_the_teardown() -> Result<(), Box<dyn StdError>> {
        let root = tempfile::tempdir()?;
        fake_environ(
            root.path(),
            "31",
            &[
                "HYPRPILOT_AGENT_SESSION=alpha",
                &format!("HYPRPILOT_AGENT_INSTANCE={NONCE}"),
            ],
        )?;

        let failure = terminate_marked_in(&Sweep {
            proc_root: root.path(),
            marker: &marker(),
            signal: &|_, _| Ok(()),
            ladder: TEST_LADDER,
        })
        .err()
        .ok_or("an immortal process was reported as swept")?;

        assert_eq!(failure.what, "agent desktop");
        assert!(failure.actual.contains("[31]"), "{}", failure.actual);
        // Both markers are named, so the refusal says what was looked for.
        assert!(
            failure.expected.contains("HYPRPILOT_AGENT_SESSION=alpha"),
            "{}",
            failure.expected
        );
        assert!(
            failure
                .expected
                .contains(&format!("HYPRPILOT_AGENT_INSTANCE={NONCE}")),
            "{}",
            failure.expected
        );
        Ok(())
    }

    #[test]
    fn a_sweep_that_cannot_read_proc_fails_instead_of_reporting_success()
    -> Result<(), Box<dyn StdError>> {
        let failure = terminate_marked_in(&Sweep {
            proc_root: Path::new("/nonexistent/proc"),
            marker: &marker(),
            signal: &|_, _| Ok(()),
            ladder: TEST_LADDER,
        })
        .err()
        .ok_or("an unreadable /proc must not count as an empty desktop")?;

        assert!(failure.actual.contains("scanning"), "{}", failure.actual);
        Ok(())
    }

    /// Fast enough to keep the escalation tests well under a second, and still
    /// ordered `dispatch exit` → `SIGTERM` → `SIGKILL` → give up.
    const TEST_LADDER: Escalation = Escalation {
        polite: Duration::from_millis(20),
        term: Duration::from_millis(20),
        kill: Duration::from_millis(20),
        poll: Duration::from_millis(2),
    };

    /// A nested compositor that dies when it is asked politely, unless `deaf`,
    /// and when signalled, unless the signal is in `ignores`.
    struct Fake {
        /// `dispatch exit` itself fails: the instance socket is already gone.
        refuses: bool,
        deaf: bool,
        ignores: &'static [&'static str],
        /// `kill` fails, as it does when the pid vanishes between two probes.
        kill_fails: bool,
        alive: Cell<bool>,
        requests: Cell<u32>,
        signals: RefCell<Vec<&'static str>>,
    }

    impl Default for Fake {
        fn default() -> Self {
            Self {
                refuses: false,
                deaf: false,
                ignores: &[],
                kill_fails: false,
                alive: Cell::new(true),
                requests: Cell::new(0),
                signals: RefCell::new(Vec::new()),
            }
        }
    }

    impl Fake {
        fn request(&self) -> Result<(), Error> {
            self.requests.set(self.requests.get() + 1);
            if self.refuses {
                return Err(Error::Tool {
                    command: "hyprctl -i beef_1700000000 dispatch exit".to_owned(),
                    message: "no such instance".to_owned(),
                });
            }
            if !self.deaf {
                self.alive.set(false);
            }
            Ok(())
        }

        fn signal(&self, signal: &'static str) -> Result<(), Error> {
            self.signals.borrow_mut().push(signal);
            if self.kill_fails {
                return Err(Error::Tool {
                    command: format!("kill -s {signal} 4242"),
                    message: "no such process".to_owned(),
                });
            }
            if !self.ignores.contains(&signal) {
                self.alive.set(false);
            }
            Ok(())
        }

        fn stopped(&self) -> Result<Vec<String>, Error> {
            stop_instance(
                &Exit {
                    session: "alpha",
                    label: "nested compositor beef_1700000000 (pid 4242)".to_owned(),
                    request: &|| self.request(),
                    alive: &|| self.alive.get(),
                    signal: &|signal| self.signal(signal),
                },
                TEST_LADDER,
            )
        }
    }

    fn live_instance() -> Instance {
        Instance::Live {
            signature: "beef_1700000000".to_owned(),
            wayland_display: "wayland-3".to_owned(),
            pid: 4242,
            console_address: "0xc0ff33".to_owned(),
        }
    }

    fn agent_state(instance: Instance, active: Option<&str>) -> Isolated {
        Isolated {
            output: "hyprpilot-alpha".to_owned(),
            workspace: "agent-alpha".to_owned(),
            instance_nonce: NONCE.to_owned(),
            size: [1920, 1080],
            shown: false,
            active_address: active.map(str::to_owned),
            instance,
            host: Vec::new(),
        }
    }

    #[test]
    fn teardown_plan_follows_the_instance_stage() {
        let pending = agent_state(Instance::Pending, Some("0xapp"));
        assert_eq!(
            teardown_plan(&pending),
            TeardownPlan {
                close: None,
                instance: None
            },
            "an output-only session has nothing to exit and nothing to close"
        );

        // A compositor that registered itself but never named its console: there
        // is a signature to exit and a runtime directory to remove, and nothing
        // else. Without this stage the state would say `Pending`, and that
        // directory would have no name left to remove it by (fact §2.9).
        let spawned = agent_state(
            Instance::Spawned {
                signature: "beef_1700000000".to_owned(),
            },
            None,
        );
        assert_eq!(
            teardown_plan(&spawned),
            TeardownPlan {
                close: None,
                instance: Some(Compositor::Spawned {
                    signature: "beef_1700000000"
                }),
            }
        );
        assert_eq!(
            teardown_plan(&spawned)
                .instance
                .and_then(Compositor::console),
            None,
            "no console was ever identified, so the output removal waits on the \
             output being vacated instead"
        );

        let live = agent_state(live_instance(), Some("0xapp"));
        assert_eq!(
            teardown_plan(&live),
            TeardownPlan {
                close: Some("0xapp"),
                instance: Some(Compositor::Live(Registered {
                    signature: "beef_1700000000",
                    pid: 4242,
                    console: "0xc0ff33",
                    display: "wayland-3",
                })),
            }
        );

        let launched_nothing = agent_state(live_instance(), None);
        let plan = teardown_plan(&launched_nothing);
        assert_eq!(plan.close, None, "no window recorded, nothing to close");
        assert!(plan.instance.is_some());
    }

    /// Before the `Spawned` stage a start killed between the spawn and the
    /// console wait persisted `Pending`, and the runtime directory the
    /// compositor had already created (fact §2.9) had no name left to remove it
    /// by. The stage is what gives a later `teardown` that name.
    #[test]
    fn a_start_that_fails_before_its_console_still_names_its_runtime_directory()
    -> Result<(), Box<dyn StdError>> {
        let pending = agent_state(Instance::Pending, None);
        assert_eq!(instance_signature(&pending.instance), None);

        let spawned = agent_state(
            Instance::Spawned {
                signature: "beef_1700000000".to_owned(),
            },
            None,
        );
        let signature =
            instance_signature(&spawned.instance).ok_or("a spawned compositor has a signature")?;
        assert_eq!(
            instance_dir(Path::new("/run/user/1000/hypr"), signature)?,
            Path::new("/run/user/1000/hypr/beef_1700000000")
        );
        Ok(())
    }

    /// Reverse order is the whole correctness of the unwind: a workspace has to
    /// get its name back, and a pushed-off workspace has to come home, *while*
    /// the output that caused either still exists. Removing the output first
    /// would leave the rename standing — which is the waybar defect.
    #[test]
    fn the_ledger_is_unwound_in_reverse_so_the_output_goes_last() {
        let acted = RefCell::new(Vec::new());
        let remove_output = |output: &str, _cursor: Option<(i32, i32)>| {
            acted.borrow_mut().push(format!("remove {output}"));
            Ok(session::OutputRemoval {
                notes: vec![format!("removed output {output}")],
                failure: None,
            })
        };
        let dispatch = |args: &[&str]| {
            acted.borrow_mut().push(args.join(" "));
            Ok(())
        };
        // The ledger as an isolated start writes it, in order.
        let ledger = vec![
            HostMutation::OutputCreated {
                output: "hyprpilot-alpha".to_owned(),
            },
            HostMutation::MonitorRuleSet {
                rule: "hyprpilot-alpha,1920x1080@60,auto,1".to_owned(),
            },
            HostMutation::WorkspaceRenamed {
                id: 3,
                from: "3".to_owned(),
                to: "agent-alpha".to_owned(),
            },
            HostMutation::WorkspaceRuleSet {
                rule: "agent-alpha, monitor:hyprpilot-alpha".to_owned(),
            },
        ];
        let unwound = unwind(
            &ledger::UndoEffects {
                remove_output: &remove_output,
                dispatch: &dispatch,
            },
            &ledger,
            None,
        );

        assert_eq!(
            acted.borrow().as_slice(),
            [
                "renameworkspace 3 3".to_owned(),
                "remove hyprpilot-alpha".to_owned()
            ],
            "the workspace gets its name back while its output is still there"
        );
        assert!(unwound.stopped.is_none());
        assert!(unwound.failures.is_empty());
        assert_eq!(
            unwound.leaked.len(),
            2,
            "both keywords are unretractable and both have to be named: {:?}",
            unwound.leaked
        );
        assert!(
            unwound
                .leaked
                .iter()
                .all(|leak| leak.contains("hyprctl reload")),
            "a leak with no remedy is a leak the user cannot clear: {:?}",
            unwound.leaked
        );
    }

    /// An undo that could not run at all stops the unwind: everything still
    /// ahead of it in the ledger is on the host, and the state has to stay on
    /// disk for `teardown` to resume from.
    #[test]
    fn an_undo_that_cannot_run_stops_the_unwind() -> Result<(), Box<dyn StdError>> {
        let dispatched = Cell::new(0_u32);
        let remove_output = |_: &str, _: Option<(i32, i32)>| {
            Err(Error::Tool {
                command: "hyprctl output remove".to_owned(),
                message: "output is busy".to_owned(),
            })
        };
        let dispatch = |_: &[&str]| {
            dispatched.set(dispatched.get() + 1);
            Ok(())
        };
        let ledger = vec![
            HostMutation::OutputCreated {
                output: "hyprpilot-alpha".to_owned(),
            },
            HostMutation::WorkspaceRenamed {
                id: 3,
                from: "3".to_owned(),
                to: "agent-alpha".to_owned(),
            },
        ];
        let unwound = unwind(
            &ledger::UndoEffects {
                remove_output: &remove_output,
                dispatch: &dispatch,
            },
            &ledger,
            None,
        );
        assert_eq!(dispatched.get(), 1, "the rename was undone first");
        let stopped = unwound
            .stopped
            .ok_or("an output that would not go has to stop the unwind")?;
        assert!(stopped.actual.contains("output is busy"), "{stopped:?}");
        Ok(())
    }

    #[test]
    fn the_console_is_waited_for_by_address_and_pid() -> Result<(), Box<dyn StdError>> {
        let clients: Vec<Client> = serde_json::from_str(NESTED_CLIENTS_JSON)?;
        let console = Console {
            address: "0xc0ff33",
            pid: 4242,
        };

        // Still mapped: the output stays, or `output remove` would hand the window
        // to one of the user's monitors.
        let waiting = console_reaped(&clients, console)
            .err()
            .ok_or("a console still on the host was reported as reaped")?;
        assert!(waiting.contains("agent-alpha"), "{waiting}");

        let gone = console_reaped(&[], console)?;
        assert!(gone.contains("0xc0ff33 is gone"), "{gone}");

        // Every process of the desktop is dead by the time this runs, so a live pid
        // at that address is a window that inherited a recycled pointer. Waiting
        // for it would abort the teardown for ever and leave the output behind.
        let recycled = console_reaped(
            &clients,
            Console {
                address: "0xc0ff33",
                pid: 999,
            },
        )?;
        assert!(recycled.contains("now belongs to pid 4242"), "{recycled}");
        Ok(())
    }

    #[test]
    fn a_console_that_was_never_recorded_is_waited_for_by_output() -> Result<(), Box<dyn StdError>>
    {
        let monitors = monitors()?;
        let clients: Vec<Client> = serde_json::from_str(NESTED_CLIENTS_JSON)?;

        // An `Instance::Pending` state names no console, and a spawn the start
        // could not identify still maps one: identity is then the monitor a window
        // reports. `0xc0ff33` sits on monitor 1, which is `headless-ci`.
        let occupied = output_vacated(&monitors, &clients, "headless-ci")
            .err()
            .ok_or("an output still holding a window was reported as empty")?;
        assert!(occupied.contains("0xc0ff33"), "{occupied}");

        let empty = output_vacated(&monitors, &clients, "DP-3")
            .err()
            .ok_or("DP-3 holds the two windows of monitor 0")?;
        assert!(empty.contains("0xdecoy"), "{empty}");

        assert!(output_vacated(&monitors, &[], "headless-ci")?.contains("no window is left"));
        // An output already gone needs no wait at all (§6.5).
        assert!(output_vacated(&monitors, &clients, "hyprpilot-alpha")?.contains("already gone"));
        Ok(())
    }

    #[test]
    fn the_frame_site_follows_the_console_not_the_flag() {
        let hidden = agent_state(live_instance(), Some("0xapp"));
        assert!(matches!(
            frame_site(&hidden, "0xc0ff33"),
            FrameSite::Headless {
                output: "hyprpilot-alpha",
                workspace: "agent-alpha"
            }
        ));

        let mut shown = agent_state(live_instance(), Some("0xapp"));
        shown.shown = true;
        // Shown: the invariant holds on the user's own workspace, so the check and
        // the diagnosis are about the console window, not about the headless output
        // (which is still active and empty, and would report everything as fine).
        assert!(matches!(
            frame_site(&shown, "0xc0ff33"),
            FrameSite::Shown {
                console: "0xc0ff33"
            }
        ));

        let reason = frames_reason(
            "alpha",
            &frame_site(&shown, "0xc0ff33"),
            "window 0xc0ff33 sits on workspace 2 while the agent desktop shows 5",
        );
        assert!(reason.contains("frame callbacks"), "{reason}");
        assert!(reason.contains("0xc0ff33"), "{reason}");
        assert!(reason.contains("session hide"), "{reason}");
    }

    #[test]
    fn a_pid_already_dead_is_an_idempotent_success() -> Result<(), Box<dyn StdError>> {
        let fake = Fake {
            alive: Cell::new(false),
            ..Fake::default()
        };

        let notes = fake.stopped()?;

        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("was already dead"), "{notes:?}");
        assert_eq!(fake.requests.get(), 0, "a dead pid is never asked to exit");
        assert!(
            fake.signals.borrow().is_empty(),
            "a dead pid is never signalled"
        );
        Ok(())
    }

    #[test]
    fn instance_exit_escalates_from_dispatch_exit_to_sigterm_then_sigkill()
    -> Result<(), Box<dyn StdError>> {
        let polite = Fake::default();
        let notes = polite.stopped()?;
        assert!(
            notes
                .iter()
                .any(|note| note.contains("exited on `dispatch exit`")),
            "{notes:?}"
        );
        assert!(polite.signals.borrow().is_empty(), "no signal was needed");

        let deaf = Fake {
            deaf: true,
            ..Fake::default()
        };
        let notes = deaf.stopped()?;
        assert_eq!(*deaf.signals.borrow(), vec!["TERM"]);
        assert!(
            notes.iter().any(|note| note.contains("SIGTERM sent")),
            "each escalation is reported: {notes:?}"
        );
        assert!(
            notes.iter().any(|note| note.contains("died after SIGTERM")),
            "{notes:?}"
        );

        let stubborn = Fake {
            deaf: true,
            ignores: &["TERM"],
            ..Fake::default()
        };
        let notes = stubborn.stopped()?;
        assert_eq!(*stubborn.signals.borrow(), vec!["TERM", "KILL"]);
        assert!(
            notes.iter().any(|note| note.contains("died after SIGKILL")),
            "{notes:?}"
        );

        // An instance whose socket is already gone still gets signalled.
        let socketless = Fake {
            refuses: true,
            ..Fake::default()
        };
        let notes = socketless.stopped()?;
        assert!(
            notes
                .iter()
                .any(|note| note.contains("refused `dispatch exit`")),
            "{notes:?}"
        );
        assert_eq!(*socketless.signals.borrow(), vec!["TERM"]);
        Ok(())
    }

    #[test]
    fn an_instance_surviving_sigkill_keeps_its_output() -> Result<(), Box<dyn StdError>> {
        let immortal = Fake {
            deaf: true,
            ignores: &["TERM", "KILL"],
            kill_fails: true,
            ..Fake::default()
        };

        let error = immortal
            .stopped()
            .err()
            .ok_or("an immortal instance was reported as gone")?;

        assert!(matches!(&error, Error::AgentDesktopAlive { .. }));
        let message = error.to_string();
        assert!(message.contains("survived"), "{message}");
        assert!(message.contains("pid 4242"), "{message}");
        // Removing the output while the console lives would drop it on the user.
        assert!(
            message.contains("would land on the user's desktop"),
            "{message}"
        );
        assert!(message.contains("--session alpha teardown"), "{message}");
        Ok(())
    }

    #[test]
    fn instance_signature_must_be_a_plain_directory_name() -> Result<(), Box<dyn StdError>> {
        assert_eq!(plain_signature("beef_1700000000")?, "beef_1700000000");

        for signature in ["", ".", "..", "../..", "a/b", "hypr/beef"] {
            let error = plain_signature(signature)
                .err()
                .ok_or_else(|| format!("signature {signature:?} was accepted"))?
                .to_string();
            assert!(error.contains("instance signature"), "{error}");
            assert!(error.contains("single directory name"), "{error}");
        }
        Ok(())
    }

    #[test]
    fn instance_log_is_kept_and_its_directory_removal_is_idempotent()
    -> Result<(), Box<dyn StdError>> {
        let root = tempfile::tempdir()?;
        let instance = root.path().join("beef_1700000000");
        let session_dir = root.path().join("sessions").join("alpha");
        fs::create_dir_all(&instance)?;
        fs::create_dir_all(&session_dir)?;
        fs::write(instance.join("hyprland.log"), b"nested log")?;

        let kept = keep_instance_log(&instance, &session_dir);
        assert!(kept.contains("instance.log"), "{kept}");
        assert_eq!(fs::read(session_dir.join("instance.log"))?, b"nested log");

        let removed = remove_instance_dir(&instance).map_err(|failure| failure.actual)?;
        assert!(removed.contains("removed"), "{removed}");
        assert!(!instance.exists());

        // Fact §2.9 is a leftover to clean, so a directory already gone and a log
        // that never existed are both successes (§6.5).
        let again = remove_instance_dir(&instance).map_err(|failure| failure.actual)?;
        assert!(again.contains("already absent"), "{again}");
        let missing = keep_instance_log(&instance, &session_dir);
        assert!(missing.contains("no nested log"), "{missing}");
        Ok(())
    }

    #[test]
    fn a_rolled_back_start_removes_the_runtime_directory_without_keeping_a_log()
    -> Result<(), Box<dyn StdError>> {
        // Fact §2.9: every start that got as far as a signature leaks
        // $XDG_RUNTIME_DIR/hypr/<sig>/ unless it is removed explicitly, and a
        // rollback is a start that failed.
        let root = tempfile::tempdir()?;
        let instances = root.path().join("hypr");
        let instance = instance_dir(&instances, "beef_1700000000")?;
        let session_dir = root.path().join("sessions").join("alpha");
        fs::create_dir_all(&instance)?;
        fs::create_dir_all(&session_dir)?;
        fs::write(instance.join("hyprland.log"), b"nested log")?;

        // The rollback removes the session directory a line later, so copying the
        // log into it would only delete it again.
        let (notes, failure) = clear_runtime_dir(&instance, None);
        assert!(failure.is_none(), "{failure:?}");
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("removed"), "{notes:?}");
        assert!(!instance.exists());
        assert!(!session_dir.join("instance.log").exists());

        // A teardown asks for the log, and keeps it next to the session's own.
        fs::create_dir_all(&instance)?;
        fs::write(instance.join("hyprland.log"), b"nested log")?;
        let (notes, failure) = clear_runtime_dir(&instance, Some(&session_dir));
        assert!(failure.is_none(), "{failure:?}");
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!(fs::read(session_dir.join("instance.log"))?, b"nested log");
        assert!(!instance.exists());
        Ok(())
    }

    #[test]
    fn a_capture_needs_a_live_instance_and_a_recorded_window() -> Result<(), Box<dyn StdError>> {
        // Both refusals happen before any compositor call, which is what makes
        // them assertable here.
        let pending = agent_state(Instance::Pending, None);
        let error = capture_target("alpha", &pending)
            .err()
            .ok_or("a capture of a pending instance was accepted")?
            .to_string();
        assert!(error.contains("agent desktop `alpha`"), "{error}");
        assert!(error.contains("never spawned"), "{error}");
        assert!(error.contains("--session alpha teardown"), "{error}");

        let launched_nothing = agent_state(live_instance(), None);
        let error = recorded_window("alpha", &launched_nothing)
            .err()
            .ok_or("a capture without a recorded window was accepted")?
            .to_string();
        assert!(error.contains("no window is recorded"), "{error}");
        assert!(error.contains("--session alpha teardown"), "{error}");
        Ok(())
    }

    #[test]
    fn a_target_is_resolved_among_the_clients_of_the_instance() -> Result<(), Box<dyn StdError>> {
        let clients = instance_clients()?;
        let by_title = |title| session::Criteria {
            title: Some(title),
            ..session::Criteria::default()
        };

        assert!(
            instance_match(&clients, &by_title("Missing title"))?.is_none(),
            "no match is a retry, not a window"
        );
        let unique = instance_match(&clients, &by_title("Unique title"))?
            .ok_or("one exact match resolves to that window")?;
        assert_eq!(unique.address, "0xddd");

        let error = instance_match(&clients, &by_title("Shared title"))
            .err()
            .ok_or("three matching windows were accepted")?;
        assert!(matches!(error, Error::WindowAmbiguous { .. }), "{error:?}");
        // Machine-readable last line, exactly as in shared mode.
        let message = error.to_string();
        let last_line = message.lines().next_back().ok_or("empty error message")?;
        let candidates: Vec<serde_json::Value> = serde_json::from_str(last_line)?;
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate["address"].as_str())
                .collect::<Vec<_>>(),
            vec![Some("0xaaa"), Some("0xbbb"), Some("0xccc")]
        );
        Ok(())
    }

    #[test]
    fn parking_options_of_shared_mode_are_refused_in_an_agent_desktop()
    -> Result<(), Box<dyn StdError>> {
        refuse_disposition(None)?;
        for disposition in [session::Disposition::Restore, session::Disposition::Close] {
            let error = refuse_disposition(Some(disposition))
                .err()
                .ok_or("--on-teardown was accepted in an agent desktop")?
                .to_string();
            assert!(error.contains(disposition.label()), "{error}");
            assert!(error.contains("whole agent desktop down"), "{error}");
        }

        refuse_untracked(false)?;
        let error = refuse_untracked(true)
            .err()
            .ok_or("--untracked was accepted in an agent desktop")?
            .to_string();
        assert!(error.contains("--untracked"), "{error}");
        assert!(error.contains("one window at a time"), "{error}");
        Ok(())
    }

    #[test]
    fn an_instance_whose_pid_lost_the_marker_is_dead() -> Result<(), Box<dyn StdError>> {
        let root = tempfile::tempdir()?;
        let nonce = format!("HYPRPILOT_AGENT_INSTANCE={NONCE}");
        let environ = |pid: &str, variables: &[&str]| fake_environ(root.path(), pid, variables);
        // The live nested compositor of session `alpha` carries both markers.
        environ(
            "4242",
            &["HYPRPILOT_AGENT_SESSION=alpha", &nonce, "PATH=/usr/bin"],
        )?;

        let live = agent_state(live_instance(), Some("0xapp"));
        assert_eq!(
            live_instance_in(root.path(), "alpha", &live)?,
            LiveInstance {
                signature: "beef_1700000000",
                wayland_display: "wayland-3",
                pid: 4242,
                console: "0xc0ff33",
            }
        );

        // A recycled pid: alive, but no longer this desktop's process.
        environ("4242", &["HYPRPILOT_AGENT_SESSION=beta"])?;
        let error = live_instance_in(root.path(), "alpha", &live)
            .err()
            .ok_or("a recycled pid was taken for the agent desktop")?;
        assert!(matches!(error, Error::AgentDesktopDead { .. }), "{error:?}");
        let message = error.to_string();
        assert!(message.contains("is dead"), "{message}");
        assert!(message.contains("beef_1700000000"), "{message}");
        assert!(message.contains("pid 4242"), "{message}");
        assert!(message.contains("--session alpha teardown"), "{message}");

        // A pid that is simply gone.
        fs::remove_dir_all(root.path().join("4242"))?;
        assert!(matches!(
            live_instance_in(root.path(), "alpha", &live),
            Err(Error::AgentDesktopDead { .. })
        ));

        // An unfinished start has no pid to probe at all, and says so instead.
        let pending = agent_state(Instance::Pending, None);
        let error = live_instance_in(root.path(), "alpha", &pending)
            .err()
            .ok_or("a pending instance was taken for a live one")?
            .to_string();
        assert!(error.contains("never spawned"), "{error}");
        Ok(())
    }

    #[test]
    fn show_and_hide_read_the_visibility_off_the_consoles_workspace() {
        assert_eq!(visibility("agent-alpha", "agent-alpha"), Visibility::Hidden);
        assert_eq!(visibility("2", "agent-alpha"), Visibility::Shown);
        // A workspace whose name merely starts the same is the user's.
        assert_eq!(visibility("agent-alpha2", "agent-alpha"), Visibility::Shown);
    }

    #[test]
    fn a_show_is_idempotent_only_where_the_user_is_actually_looking() {
        let focused = |name: &str, monitor: &str| FocusedWorkspace {
            name: name.to_owned(),
            monitor: monitor.to_owned(),
        };

        assert!(shown_where_the_user_looks(
            "2",
            &focused("2", "DP-3"),
            "hyprpilot-alpha"
        ));
        // Shown, but on a workspace the user has switched away from: the console
        // is occluded, so `show` has work to do (fact §2.2).
        assert!(!shown_where_the_user_looks(
            "2",
            &focused("5", "DP-3"),
            "hyprpilot-alpha"
        ));
        // A waybar click focused the agent desktop's own headless output (§7):
        // the console being on that workspace is hidden, not shown.
        assert!(!shown_where_the_user_looks(
            "agent-alpha",
            &focused("agent-alpha", "hyprpilot-alpha"),
            "hyprpilot-alpha"
        ));
    }

    #[test]
    fn a_focus_on_the_agent_output_is_not_a_destination() -> Result<(), Box<dyn StdError>> {
        let state = agent_state(live_instance(), Some("0xapp"));
        let monitors = monitors()?;
        let focused = |name: &str, monitor: &str| FocusedWorkspace {
            name: name.to_owned(),
            monitor: monitor.to_owned(),
        };
        assert_eq!(
            user_workspace(&focused("1", "DP-3"), &monitors, "alpha", &state)?,
            "1"
        );

        for (name, monitor) in [("agent-alpha", "hyprpilot-alpha"), ("3", "hyprpilot-alpha")] {
            let error = user_workspace(&focused(name, monitor), &monitors, "alpha", &state)
                .err()
                .ok_or("the agent desktop's own output was accepted as a destination")?
                .to_string();
            assert!(error.contains("hyprpilot-alpha"), "{error}");
            assert!(error.contains("waybar"), "{error}");
            assert!(error.contains("session show"), "{error}");
        }

        // `hyprctl activeworkspace` answers with the workspace *underneath* an
        // open special workspace: a console moved there arrives occluded, which
        // freezes the nested compositor (fact §2.2).
        let mut covered = monitors.clone();
        covered[0].special_workspace = "special:magic".to_owned();
        let error = user_workspace(&focused("1", "DP-3"), &covered, "alpha", &state)
            .err()
            .ok_or("a workspace under a special workspace was accepted")?
            .to_string();
        assert!(error.contains("special:magic"), "{error}");
        assert!(error.contains("stop receiving frames"), "{error}");

        // A focused monitor `hyprctl monitors` does not list is a disagreement to
        // report, not something to move a window on.
        let error = user_workspace(&focused("1", "DP-9"), &monitors, "alpha", &state)
            .err()
            .ok_or("an unknown monitor was accepted as a destination")?
            .to_string();
        assert!(error.contains("does not list"), "{error}");
        Ok(())
    }

    #[test]
    fn a_console_move_is_verified_against_what_the_compositor_reports()
    -> Result<(), Box<dyn StdError>> {
        let clients: Vec<Client> = serde_json::from_str(NESTED_CLIENTS_JSON)?;
        let console = clients.first().ok_or("empty nested clients fixture")?;

        console_settled(
            console,
            &ConsoleWant {
                workspace: "agent-alpha",
                floating: Some(false),
                at: Some([5120, 0]),
            },
        )?;
        // What was not asked for is not checked.
        console_settled(
            console,
            &ConsoleWant {
                workspace: "agent-alpha",
                floating: None,
                at: None,
            },
        )?;

        for (want, expected) in [
            (
                ConsoleWant {
                    workspace: "2",
                    floating: None,
                    at: None,
                },
                "on workspace agent-alpha",
            ),
            (
                ConsoleWant {
                    workspace: "agent-alpha",
                    floating: Some(true),
                    at: None,
                },
                "floating is false",
            ),
            (
                ConsoleWant {
                    workspace: "agent-alpha",
                    floating: None,
                    at: Some([0, 0]),
                },
                "at [5120, 0]",
            ),
        ] {
            let observed = console_settled(console, &want)
                .err()
                .ok_or("a console that had not moved was accepted")?;
            assert_eq!(observed, expected);
        }
        Ok(())
    }

    #[test]
    fn the_shown_flag_is_written_from_reality_and_idempotent_otherwise()
    -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        let mut state = agent_state(live_instance(), Some("0xapp"));

        // Already hidden: a success that says so, and no write.
        let message = persist_visibility(
            "alpha",
            &path,
            &mut state,
            Visibility::Hidden,
            "already hidden".to_owned(),
        )?;
        assert_eq!(message, "already hidden");
        assert!(!state.shown);
        assert!(
            !path.exists(),
            "an idempotent hide must not rewrite the state"
        );

        persist_visibility(
            "alpha",
            &path,
            &mut state,
            Visibility::Shown,
            "shown".to_owned(),
        )?;
        assert!(state.shown);
        let raw = fs::read_to_string(&path)?;
        assert!(raw.contains("\"shown\": true"), "{raw}");
        assert!(raw.contains("\"mode\": \"isolated\""), "{raw}");
        assert!(raw.contains("\"name\": \"alpha\""), "{raw}");

        let written = fs::metadata(&path)?.modified()?;
        let message = persist_visibility(
            "alpha",
            &path,
            &mut state,
            Visibility::Shown,
            "already shown".to_owned(),
        )?;
        assert_eq!(message, "already shown");
        assert_eq!(
            fs::metadata(&path)?.modified()?,
            written,
            "a second show must not rewrite the state"
        );
        Ok(())
    }

    #[test]
    fn a_frozen_agent_desktop_names_the_invariant_and_the_host_fallback() {
        let reason = frozen_reason(
            "alpha",
            "hyprpilot-alpha",
            "agent-alpha",
            "workspace 3 is active on hyprpilot-alpha, not agent-alpha",
        );

        // Fact §2.2 in the message, not a guess about what went wrong.
        assert!(reason.contains("frame callbacks"), "{reason}");
        assert!(reason.contains("agent-alpha"), "{reason}");
        assert!(reason.contains("workspace 3 is active"), "{reason}");
        // The documented fallback of §5.
        assert!(reason.contains("grim -o hyprpilot-alpha"), "{reason}");
        assert!(reason.contains("--session alpha teardown"), "{reason}");
    }

    #[test]
    fn a_workspace_the_user_owns_is_never_renamed() -> Result<(), Box<dyn StdError>> {
        let clients = instance_clients()?;
        assert_eq!(
            workspace_occupants(&clients, "1"),
            vec!["0xaaa (`Shared title`)".to_owned()]
        );
        assert!(workspace_occupants(&clients, "9").is_empty());

        // A scratch pad the user keeps: empty, active nowhere, and still theirs.
        // The ledger gives a renamed workspace its name back at teardown, but
        // only at teardown — for the whole life of the session the user would be
        // looking at a slot of theirs wearing an `agent-*` label (§12.1).
        let host = snapshot_with(&[("DP-3", "1"), ("HDMI-A-1", "8")], &["7"], Some("0xabc"));
        // The workspace a fresh headless output brings with it: empty, not one the
        // user was looking at, and absent from the snapshot taken before the
        // output existed.
        renameable(&host, &clients, "9")?;

        let occupied = renameable(&host, &clients, "2")
            .err()
            .ok_or("a workspace holding a window was accepted")?;
        assert!(occupied.contains("0xbbb"), "{occupied}");

        let visible = renameable(&host, &clients, "8")
            .err()
            .ok_or("a workspace the user was looking at was accepted")?;
        assert!(visible.contains("visible on HDMI-A-1"), "{visible}");

        let scratch = renameable(&host, &clients, "7")
            .err()
            .ok_or("an empty workspace the user already had was accepted")?;
        assert!(scratch.contains("already existed"), "{scratch}");
        Ok(())
    }
}
