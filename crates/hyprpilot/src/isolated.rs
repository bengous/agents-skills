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
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{Error, RestoreFailure};
use crate::hypr::{self, Ctl};
use crate::session::{self, Instance, Isolated, ModeState, Session};

/// Injected into the nested compositor's environment at spawn and refused at
/// the top of `start`: an output created *inside* a nested Hyprland stays 0x0
/// (fact §2.7), so this machinery must only ever run on the user's session.
/// Every process of an agent desktop inherits it, which is also how a failed
/// start knows what to kill.
pub const AGENT_SESSION_ENV: &str = "HYPRPILOT_AGENT_SESSION";
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
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
/// §6.2, on the registered pid: `dispatch exit` first, then `SIGTERM`, then
/// `SIGKILL`, then the teardown refuses to remove the output.
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

/// Host resources acquired so far, so a failure undoes exactly what exists.
#[derive(Default)]
struct Acquired {
    output: bool,
    instance: Option<Live>,
}

/// What the start must leave untouched (§4.6): the workspace active on every
/// host output and the user's focused window. The cursor is where
/// `output remove` has to warp back to (fact §2.8).
struct HostSnapshot {
    workspaces: BTreeMap<String, String>,
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
    refuse_nested_marker(agent_session_marker().as_deref())?;
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
    start.claim()?;

    let mut acquired = Acquired::default();
    match start.build(&mut acquired, &host) {
        Ok(message) => Ok(message),
        Err(error) => Err(start.rolled_back(error, &acquired, &host)),
    }
}

impl Start<'_> {
    /// Steps 2 to 7 of §4, each persisting what it acquired before moving on.
    fn build(&self, acquired: &mut Acquired, host: &HostSnapshot) -> Result<String, Error> {
        self.create_output()?;
        acquired.output = true;
        self.persist(Instance::Pending, None)?;

        self.rename_workspace(host)?;
        let config = self.write_config()?;

        let live = self.spawn_instance(&config)?;
        acquired.instance = Some(live.clone());
        self.persist(live.instance(), None)?;

        check_host(host)?;

        let window = self.launch_app(&live.signature)?;
        self.persist(live.instance(), Some(window.address.clone()))?;
        self.wait_until_ready(&live, &window.address)?;
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

    fn state(&self, instance: Instance, active_address: Option<String>) -> Session {
        Session {
            schema_version: session::SCHEMA_VERSION,
            name: self.name.to_owned(),
            state: ModeState::Isolated(Isolated {
                output: self.output.clone(),
                workspace: self.workspace.clone(),
                size: self.size,
                shown: false,
                active_address,
                instance,
            }),
        }
    }

    /// The atomic claim (§4.1): the state already names every resource this
    /// start will acquire, so `teardown` can clean up from here onwards.
    fn claim(&self) -> Result<(), Error> {
        session::save_new_to(&self.path, &self.state(Instance::Pending, None))
    }

    fn persist(&self, instance: Instance, active_address: Option<String>) -> Result<(), Error> {
        session::save_over(&self.path, &self.state(instance, active_address))
    }

    /// §4.2. Resolution *and* scale are imposed: a headless output otherwise
    /// inherits a non-trivial scale (fact §2.10).
    fn create_output(&self) -> Result<(), Error> {
        hypr::output_create_headless(&self.output)?;
        hypr::keyword_monitor(&self.output, self.size[0], self.size[1])?;
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
    fn rename_workspace(&self, host: &HostSnapshot) -> Result<(), Error> {
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

        hypr::dispatch(&["renameworkspace", &current.id.to_string(), &self.workspace])?;
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
    fn spawn_instance(&self, config: &Path) -> Result<Live, Error> {
        let log = self.dir.join(NESTED_LOG_FILE);
        let command = spawn_command(self.name, &self.workspace, config, &log)?;
        let instances = instances_dir()?;
        let runtime = runtime_root()?;
        let signatures_before = dir_entries(&instances)?;
        let sockets_before = wayland_sockets(&runtime)?;
        let windows_before = hypr::clients()?
            .into_iter()
            .map(|client| client.address)
            .collect::<BTreeSet<_>>();

        hypr::dispatch(&["exec", &command])?;

        let signature = wait_for_new_entry(
            &instances,
            &signatures_before,
            dir_entries,
            "a new Hyprland instance",
            &log,
        )?;
        let console = self.wait_for_console(&windows_before, &log)?;
        let wayland_display = wait_for_new_entry(
            &runtime,
            &sockets_before,
            wayland_sockets,
            "a new Wayland socket",
            &log,
        )?;
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

    fn wait_for_console(
        &self,
        before: &BTreeSet<String>,
        log: &Path,
    ) -> Result<hypr::Client, Error> {
        poll_until(
            session::WINDOW_APPEAR_TIMEOUT,
            || {
                let clients = hypr::clients()?;
                let ours = |pid: i32| process_carries_marker(pid, self.name);
                let console = select_console(&clients, before, &ours)?;
                Ok(console
                    .cloned()
                    .ok_or_else(|| format!("{} host windows, none of them ours", clients.len())))
            },
            |observed| {
                format!(
                    "the console window of agent desktop `{name}` (class {CONSOLE_CLASS}, \
                     {AGENT_SESSION_ENV}={name}) (last observed: {observed}); nested log: {}",
                    log.display(),
                    name = self.name,
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

    /// Undoes what exists, in the order that never drops a live window onto the
    /// user's desktop, and reports the original failure alongside whatever the
    /// rollback could not undo.
    fn rolled_back(&self, error: Error, acquired: &Acquired, host: &HostSnapshot) -> Error {
        let restore = self.rollback(acquired, host);
        if restore.is_empty() {
            error
        } else {
            Error::Guarded {
                action: Some(Box::new(error)),
                restore,
            }
        }
    }

    fn rollback(&self, acquired: &Acquired, host: &HostSnapshot) -> Vec<RestoreFailure> {
        if let Err(failure) = self.terminate(acquired) {
            // The console still lives on the headless output: removing that
            // output would drop the window onto the user's desktop, and the
            // state has to survive for `teardown`.
            return vec![failure];
        }
        let mut failures = Vec::new();
        if acquired.output {
            // Same cursor-preserving removal as either mode's teardown
            // (fact §2.8); the snapshot is the fallback if `cursorpos` cannot be
            // read at the last moment.
            match session::remove_output_restoring_cursor(&self.output, Some(host.cursor)) {
                Ok(removal) => failures.extend(removal.failure),
                Err(error) => {
                    return vec![RestoreFailure {
                        what: "agent output",
                        expected: format!("{} removed", self.output),
                        actual: error.to_string(),
                    }];
                }
            }
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

    fn terminate(&self, acquired: &Acquired) -> Result<(), RestoreFailure> {
        if let Some(live) = &acquired.instance {
            // Politeness first; the marker sweep below is what guarantees the
            // desktop is gone, including a compositor spawned but never
            // identified.
            let _ = hypr::dispatch_on(Ctl::Instance(&live.signature), &["exit"]);
        }
        terminate_marked(self.name)
    }
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
}

/// What §6 has to undo, decided from the state alone so it can be asserted
/// without a compositor. Steps 4 (output and cursor) and 5 (state) are
/// unconditional and belong to `session::finish_teardown`.
#[derive(Debug, PartialEq, Eq)]
struct TeardownPlan<'a> {
    /// Window to close politely, inside the instance (step 1).
    close: Option<&'a str>,
    /// Compositor to exit, then whose runtime directory to clear (steps 2, 3).
    instance: Option<Registered<'a>>,
}

fn teardown_plan(isolated: &Isolated) -> TeardownPlan<'_> {
    let instance = match &isolated.instance {
        Instance::Pending => None,
        Instance::Live {
            signature,
            pid,
            console_address,
            ..
        } => Some(Registered {
            signature,
            pid: *pid,
            console: console_address,
        }),
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
    let mut notes = Vec::new();
    let mut failures = Vec::new();

    match plan.instance {
        None => notes.push("nested compositor was never spawned".to_owned()),
        Some(instance) => {
            if let Some(address) = plan.close {
                notes.push(close_app(instance.signature, address));
            }
            notes.extend(stop_registered(&instance, session)?);
            let (cleared, failure) =
                clear_instance_runtime(instance.signature, &session::session_dir(session)?);
            notes.extend(cleared);
            failures.extend(failure);
        }
    }

    // Whatever stage the state described, no process of this desktop may outlive
    // the teardown: the environment marker is the authority, so a compositor
    // spawned but never recorded is caught here too. Its output is removed right
    // after this, hence the refusal rather than a note.
    if let Err(failure) = terminate_marked(session) {
        return Err(Error::AgentDesktopAlive {
            session: session.to_owned(),
            detail: failure.actual,
        });
    }
    notes.push("no process of the agent desktop left".to_owned());

    // The console is a *host* window, and the host reaps it when the compositor
    // that owns it dies. Removing the output before that would hand the window to
    // one of the user's outputs for as long as it survives.
    if let Some(instance) = plan.instance {
        notes.push(wait_console_reaped(instance.console)?);
    }

    Ok(Teardown { notes, failures })
}

/// Bounded, and a timeout aborts the teardown rather than removing an output a
/// window still sits on — the same rule the shared teardown follows before it
/// removes its own output. A window already gone is an immediate success (§6.5).
fn wait_console_reaped(address: &str) -> Result<String, Error> {
    poll_until(
        session::WINDOW_PLACE_TIMEOUT,
        || {
            Ok(find_client(&hypr::clients()?, address).map_or_else(
                || Ok(format!("console window {address} is gone from the host")),
                |console| Err(format!("still on workspace {}", console.workspace.name)),
            ))
        },
        |observed| {
            format!(
                "the host to reap console window {address} now that its compositor is dead (last \
                 observed: {observed}); its output stays until then"
            )
        },
    )
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
fn close_settled(ctl: Ctl<'_>, address: &str) -> String {
    let deadline = Instant::now() + POLITE_CLOSE_TIMEOUT;
    loop {
        match hypr::clients_on(ctl) {
            Ok(clients) if find_client(&clients, address).is_none() => {
                return format!("closed window {address} in the agent desktop");
            }
            Ok(_) => {}
            Err(error) => return format!("closed window {address}, then listing failed ({error})"),
        }
        if Instant::now() >= deadline {
            return format!(
                "asked window {address} to close, still mapped after {}ms",
                POLITE_CLOSE_TIMEOUT.as_millis()
            );
        }
        thread::sleep(session::POLL_INTERVAL);
    }
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

fn stop_registered(instance: &Registered<'_>, session: &str) -> Result<Vec<String>, Error> {
    let ctl = Ctl::Instance(instance.signature);
    let pid = instance.pid;
    stop_instance(
        &Exit {
            session,
            label: format!("nested compositor {} (pid {pid})", instance.signature),
            request: &|| hypr::dispatch_on(ctl, &["exit"]),
            // The marker is both liveness and identity: a recycled pid no longer
            // carries it, so it is never signalled.
            alive: &|| pid_carries_marker(pid, session),
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
fn instance_dir(signature: &str) -> Result<PathBuf, Error> {
    Ok(instances_dir()?.join(plain_signature(signature)?))
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
fn clear_instance_runtime(
    signature: &str,
    session_dir: &Path,
) -> (Vec<String>, Option<RestoreFailure>) {
    let dir = match instance_dir(signature) {
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
    let mut notes = vec![keep_instance_log(&dir, session_dir)];
    match remove_instance_dir(&dir) {
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

/// A fresh headless output gets a workspace of its own, and only such a
/// workspace may be renamed: one holding windows, or one the user was looking at
/// a moment ago, belongs to them.
fn renameable(
    host: &HostSnapshot,
    clients: &[hypr::Client],
    workspace: &str,
) -> Result<(), String> {
    let occupants = workspace_occupants(clients, workspace);
    if !occupants.is_empty() {
        return Err(format!("holds {}", occupants.join(", ")));
    }
    match host
        .workspaces
        .iter()
        .find(|(_, active)| *active == workspace)
    {
        Some((output, _)) => Err(format!("it was visible on {output} a moment ago")),
        None => Ok(()),
    }
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
    if binary_on_path(NESTED_BINARY) {
        return Ok(());
    }
    Err(Error::Tool {
        command: NESTED_BINARY.to_owned(),
        message: format!(
            "{NESTED_BINARY} not found on PATH — an agent desktop is a nested Hyprland"
        ),
    })
}

fn binary_on_path(binary: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|path| env::split_paths(&path).any(|dir| dir.join(binary).is_file()))
}

pub fn nested_binary_present() -> bool {
    binary_on_path(NESTED_BINARY)
}

/// Hyprland hands the command to `sh -c`, so the paths are quoted and a path
/// that could close a quote is refused rather than escaped.
fn spawn_command(
    session: &str,
    workspace: &str,
    config: &Path,
    log: &Path,
) -> Result<String, Error> {
    let config = shell_path(config)?;
    let log = shell_path(log)?;
    Ok(format!(
        "[workspace name:{workspace} silent; noinitialfocus; fullscreen] \
         env {AGENT_SESSION_ENV}={session} {NESTED_BINARY} -c '{config}' > '{log}' 2>&1"
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

fn runtime_root() -> Result<PathBuf, Error> {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or(Error::Env("XDG_RUNTIME_DIR"))
}

fn instances_dir() -> Result<PathBuf, Error> {
    Ok(runtime_root()?.join("hypr"))
}

fn dir_entries(dir: &Path) -> Result<BTreeSet<String>, Error> {
    let context = || format!("listing {}", dir.display());
    let entries = fs::read_dir(dir).map_err(|source| Error::Io {
        context: context(),
        source,
    })?;
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            context: context(),
            source,
        })?;
        names.insert(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

fn wayland_sockets(dir: &Path) -> Result<BTreeSet<String>, Error> {
    Ok(dir_entries(dir)?
        .into_iter()
        .filter(|name| is_wayland_socket(name))
        .collect())
}

/// A Wayland socket is `wayland-<n>`; its lock file is `wayland-<n>.lock`, and
/// no socket name carries a dot.
fn is_wayland_socket(name: &str) -> bool {
    name.starts_with("wayland-") && !name.contains('.')
}

fn new_entries(before: &BTreeSet<String>, now: &BTreeSet<String>) -> Vec<String> {
    now.difference(before).cloned().collect()
}

/// Bounded diff of a runtime directory: exactly one new entry is ours, several
/// mean a concurrent birth this start refuses to guess between.
fn wait_for_new_entry(
    dir: &Path,
    before: &BTreeSet<String>,
    list: impl Fn(&Path) -> Result<BTreeSet<String>, Error>,
    what: &str,
    log: &Path,
) -> Result<String, Error> {
    poll_until(
        INSTANCE_APPEAR_TIMEOUT,
        || match new_entries(before, &list(dir)?).as_slice() {
            [] => Ok(Err("none".to_owned())),
            [entry] => Ok(Ok(entry.clone())),
            several => Err(Error::Tool {
                command: format!("listing {}", dir.display()),
                message: format!(
                    "{} new entries appeared ({}) — cannot tell which one belongs to this start; \
                     teardown and retry",
                    several.len(),
                    several.join(", ")
                ),
            }),
        },
        |observed| {
            format!(
                "{what} in {} (last observed: {observed}); nested log: {}",
                dir.display(),
                log.display()
            )
        },
    )
}

/// The console is a window that did not exist before the spawn, carries the
/// nested compositor's class (fact §2.5) and whose process carries our marker.
/// The title is never part of the identity.
fn select_console<'a>(
    clients: &'a [hypr::Client],
    before: &BTreeSet<String>,
    ours: &dyn Fn(i32) -> Result<bool, Error>,
) -> Result<Option<&'a hypr::Client>, Error> {
    let mut console: Option<&hypr::Client> = None;
    for client in clients {
        if client.class != CONSOLE_CLASS || before.contains(&client.address) {
            continue;
        }
        if !ours(client.pid)? {
            continue;
        }
        if let Some(other) = console {
            return Err(Error::Tool {
                command: "hyprctl clients".to_owned(),
                message: format!(
                    "two console windows carry {AGENT_SESSION_ENV} for this session: {} (pid {}) \
                     and {} (pid {}) — teardown and retry",
                    other.address, other.pid, client.address, client.pid
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
    window_frame(outputs, clients, address).map(|_| ())
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

/// Instance side: the window is mapped and the nested compositor is showing the
/// workspace it sits on. Returns what a capture needs to frame it.
fn window_frame<'a>(
    outputs: &'a [hypr::Monitor],
    clients: &'a [hypr::Client],
    address: &str,
) -> Result<(&'a hypr::Client, &'a hypr::Monitor), String> {
    let Some(window) = find_client(clients, address) else {
        return Err(format!("window {address} is gone from the agent desktop"));
    };
    if window.size[0] <= 0 || window.size[1] <= 0 {
        return Err(format!(
            "window {address} is not mapped (size {:?})",
            window.size
        ));
    }
    let Some(monitor) = outputs.iter().find(|monitor| monitor.id == window.monitor) else {
        return Err(format!(
            "window {address} reports monitor {} which the agent desktop does not have",
            window.monitor
        ));
    };
    if monitor.active_workspace.name != window.workspace.name {
        return Err(format!(
            "window {address} sits on workspace {} while the agent desktop shows {}",
            window.workspace.name, monitor.active_workspace.name
        ));
    }
    if !monitor.special_workspace.is_empty() {
        return Err(format!(
            "special workspace {} occludes the agent desktop",
            monitor.special_workspace
        ));
    }
    Ok((window, monitor))
}

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
    let Instance::Live {
        signature,
        wayland_display,
        pid,
        console_address,
    } = &isolated.instance
    else {
        return Err(Error::AgentDesktopUnready {
            session: session.to_owned(),
            reason: format!(
                "its nested compositor was never spawned (`session start --isolated` did not \
                 finish) — run `hyprpilot --session {session} teardown` and start it again"
            ),
        });
    };
    // The marker is liveness *and* identity: a dead pid has no readable
    // environment and a recycled one no longer carries this session's marker, so
    // no command can address a stranger's process.
    if !pid_carries_marker_in(proc_root, *pid, session) {
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
    // Fact §2.2: a console window the host stopped compositing freezes the
    // nested compositor, and screencopy then blocks for ever. `session show`
    // moves the console to a workspace of the user's own, where the same
    // guarantee comes from the host showing that workspace (§5).
    if !isolated.shown
        && let Err(observed) =
            host_frames(&hypr::monitors()?, &isolated.output, &isolated.workspace)
    {
        return Err(unready(frozen_reason(
            session,
            &isolated.output,
            &isolated.workspace,
            &observed,
        )));
    }

    let ctl = instance.ctl();
    let outputs = hypr::monitors_on(ctl)?;
    let clients = hypr::clients_on(ctl)?;
    let (window, monitor) = window_frame(&outputs, &clients, address).map_err(unready)?;
    Ok(AgentCapture {
        wayland_display: instance.wayland_display.to_owned(),
        window: window.clone(),
        monitor: monitor.clone(),
    })
}

/// Fact §2.2, spelled out for the user: the one documented cause of a frozen
/// agent desktop, with the documented host-side fallback.
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
pub fn frozen_observation(host_output: &str, workspace: &str) -> String {
    match hypr::monitors() {
        Ok(monitors) => host_frames(&monitors, host_output, workspace)
            .err()
            .unwrap_or_else(|| {
                format!(
                    "workspace {workspace} is still active on {host_output}, so the block is \
                     elsewhere"
                )
            }),
        Err(error) => format!("reading `hyprctl monitors` failed: {error}"),
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
    let destination = user_workspace(&focused, session, isolated)?;

    // Floating first: changing the floating mode is what drops the fullscreen
    // state the start's one-shot rule set, and a fullscreen console would
    // otherwise cover the user's whole monitor. The size is then pinned to what
    // the console had, because the agent desktop renders at the size of this
    // window: letting Hyprland pick a floating size would silently change the
    // resolution the agent has been working in.
    let size = console.size;
    hypr::dispatch(&["setfloating", &window_arg(&console_address)])?;
    resize_console(&console_address, size)?;
    hypr::dispatch(&[
        "movetoworkspacesilent",
        &format!(
            "{},address:{console_address}",
            session::workspace_selector(&destination)
        ),
    ])?;

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

/// Every process of an agent desktop inherits the marker, including the app the
/// nested compositor launched, so the environment identifies the whole desktop.
/// A process is never identified by its binary name.
fn marked_pids_in(proc_root: &Path, session: &str) -> Result<Vec<u32>, Error> {
    let context = || format!("listing {}", proc_root.display());
    let entries = fs::read_dir(proc_root).map_err(|source| Error::Io {
        context: context(),
        source,
    })?;
    let mut pids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            context: context(),
            source,
        })?;
        let Some(pid) = parse_pid(&entry.file_name()) else {
            continue;
        };
        if environ_carries_marker(&entry.path(), session) {
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
fn environ_carries_marker(proc_dir: &Path, session: &str) -> bool {
    let needle = format!("{AGENT_SESSION_ENV}={session}");
    fs::read(proc_dir.join("environ")).is_ok_and(|environ| {
        environ
            .split(|byte| *byte == 0)
            .any(|variable| variable == needle.as_bytes())
    })
}

fn process_carries_marker(pid: i32, session: &str) -> Result<bool, Error> {
    let pid = u32::try_from(pid).map_err(|_| Error::Tool {
        command: "hyprctl clients".to_owned(),
        message: format!("client reports invalid pid {pid}"),
    })?;
    Ok(pid_carries_marker(pid, session))
}

/// Liveness *and* identity of a recorded pid: a dead pid has no readable
/// environment, and a recycled one no longer carries this session's marker.
fn pid_carries_marker(pid: u32, session: &str) -> bool {
    pid_carries_marker_in(Path::new("/proc"), pid, session)
}

fn pid_carries_marker_in(proc_root: &Path, pid: u32, session: &str) -> bool {
    environ_carries_marker(&proc_root.join(pid.to_string()), session)
}

/// `SIGTERM` while the grace period lasts, then `SIGKILL`; bounded either way.
fn terminate_marked(session: &str) -> Result<(), RestoreFailure> {
    let started = Instant::now();
    let failure = |actual: String| RestoreFailure {
        what: "agent desktop",
        expected: format!("no process left carrying {AGENT_SESSION_ENV}={session}"),
        actual,
    };
    loop {
        let pids = marked_pids_in(Path::new("/proc"), session)
            .map_err(|error| failure(format!("scanning /proc failed: {error}")))?;
        if pids.is_empty() {
            return Ok(());
        }
        if started.elapsed() >= EXIT_TIMEOUT {
            return Err(failure(format!("pids {pids:?} still alive")));
        }
        let signal = if started.elapsed() >= EXIT_GRACE {
            "KILL"
        } else {
            "TERM"
        };
        for pid in pids {
            let _ = session::signal_process(pid, signal);
        }
        thread::sleep(session::POLL_INTERVAL);
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
        AGENT_SESSION_ENV, ConsoleWant, Exit, HostSnapshot, Keymap, LiveInstance, Registered,
        TeardownPlan, Visibility, active_workspaces, capturable, capture_target, console_settled,
        deviation, dir_entries, ensure_output_absent, frozen_reason, instance_match,
        is_wayland_socket, keep_instance_log, keymap_of, live_instance_in, marked_pids_in,
        nested_config, new_entries, output_is_configured, persist_visibility, plain_signature,
        recorded_window, refuse_disposition, refuse_nested_marker, refuse_untracked,
        remove_instance_dir, renameable, select_console, shell_path, shown_where_the_user_looks,
        spawn_command, stop_instance, teardown_plan, user_workspace, visibility, wayland_sockets,
        workspace_occupants,
    };
    use crate::error::Error;
    use crate::hypr::{Client, Devices, FocusedWorkspace, Monitor};
    use crate::session::{self, Escalation, Instance, Isolated};

    const MONITORS_JSON: &str = include_str!("../fixtures/monitors.json");
    const DEVICES_JSON: &str = include_str!("../fixtures/devices.json");
    const NESTED_CLIENTS_JSON: &str = include_str!("../fixtures/clients-nested.json");
    const INSTANCE_CLIENTS_JSON: &str = include_str!("../fixtures/clients-ambiguous.json");

    fn monitors() -> Result<Vec<Monitor>, serde_json::Error> {
        serde_json::from_str(MONITORS_JSON)
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

    fn snapshot(outputs: &[(&str, &str)], focus: Option<&str>) -> HostSnapshot {
        HostSnapshot {
            workspaces: outputs
                .iter()
                .map(|(output, workspace)| ((*output).to_owned(), (*workspace).to_owned()))
                .collect(),
            active_window: focus.map(str::to_owned),
            cursor: (100, 200),
        }
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

    #[test]
    fn instance_diff_reports_only_entries_the_spawn_created() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        // A third-party instance that already existed is never a candidate.
        fs::create_dir_all(dir.path().join("cafe_1700000000"))?;
        let before = dir_entries(dir.path())?;
        assert!(new_entries(&before, &dir_entries(dir.path())?).is_empty());

        fs::create_dir_all(dir.path().join("beef_1700000001"))?;
        assert_eq!(
            new_entries(&before, &dir_entries(dir.path())?),
            vec!["beef_1700000001".to_owned()]
        );

        fs::create_dir_all(dir.path().join("dead_1700000002"))?;
        assert_eq!(
            new_entries(&before, &dir_entries(dir.path())?),
            vec!["beef_1700000001".to_owned(), "dead_1700000002".to_owned()]
        );
        Ok(())
    }

    #[test]
    fn wayland_socket_listing_ignores_lock_files() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        for name in ["wayland-1", "wayland-1.lock", "wayland-2", "bus"] {
            fs::write(dir.path().join(name), b"")?;
        }

        assert_eq!(
            wayland_sockets(dir.path())?,
            ["wayland-1", "wayland-2"]
                .into_iter()
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        );
        assert!(is_wayland_socket("wayland-42"));
        assert!(!is_wayland_socket("wayland-42.lock"));
        assert!(!is_wayland_socket("bus"));
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

        let known = clients
            .iter()
            .map(|client| client.address.clone())
            .collect::<BTreeSet<_>>();
        assert!(
            select_console(&clients, &known, &ours)?.is_none(),
            "a window that predates the spawn cannot be our console"
        );

        let anything = |_: i32| Ok(true);
        let error = select_console(&clients, &nothing, &anything)
            .err()
            .ok_or("two marked consoles were accepted")?
            .to_string();
        assert!(error.contains("two console windows"), "{error}");
        assert!(error.contains("0xc0ff33"), "{error}");
        assert!(error.contains("0xdecoy"), "{error}");

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
    fn nested_marker_refuses_a_start_inside_an_agent_desktop() -> Result<(), Box<dyn StdError>> {
        refuse_nested_marker(None)?;

        for marker in ["alpha", ""] {
            let error = refuse_nested_marker(Some(marker))
                .err()
                .ok_or("a start inside an agent desktop was accepted")?;
            assert!(matches!(&error, Error::NestedRefused { .. }));
            let message = error.to_string();
            assert!(message.contains(AGENT_SESSION_ENV), "{message}");
            assert!(message.contains("0x0"), "{message}");
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
        let clients = instance_clients()?;

        // Host side: `headless-ci` shows `proto`. Instance side: `0xaaa` sits on
        // workspace `1`, which is what its own monitor is showing.
        capturable(
            &monitors,
            &monitors,
            &clients,
            "headless-ci",
            "proto",
            "0xaaa",
        )?;

        let frozen = capturable(
            &monitors,
            &monitors,
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
            &monitors,
            &clients,
            "hyprpilot-alpha",
            "agent-alpha",
            "0xaaa",
        )
        .err()
        .ok_or("a missing host output was accepted")?;
        assert!(absent.contains("hyprpilot-alpha is absent"), "{absent}");

        let mut occluded = monitors.clone();
        occluded[1].special_workspace = "special:magic".to_owned();
        let occluded = capturable(
            &occluded,
            &monitors,
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
        let mut clients = instance_clients()?;

        let gone = capturable(
            &monitors,
            &monitors,
            &clients,
            "headless-ci",
            "proto",
            "0xzzz",
        )
        .err()
        .ok_or("an absent window was accepted")?;
        assert!(gone.contains("0xzzz is gone"), "{gone}");

        // `0xbbb` sits on workspace `2` while its monitor shows `1`.
        let hidden = capturable(
            &monitors,
            &monitors,
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
            &monitors,
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
            &monitors,
            &clients,
            "headless-ci",
            "proto",
            "0xaaa",
        )
        .err()
        .ok_or("a window on an unknown monitor was accepted")?;
        assert!(stray.contains("monitor 7"), "{stray}");
        Ok(())
    }

    #[test]
    fn spawn_command_carries_the_one_shot_rules_marker_and_log() -> Result<(), Box<dyn StdError>> {
        let command = spawn_command(
            "alpha",
            "agent-alpha",
            Path::new("/run/user/1000/hyprpilot/sessions/alpha/hyprland.conf"),
            Path::new("/run/user/1000/hyprpilot/sessions/alpha/hyprland.log"),
        )?;

        assert_eq!(
            command,
            "[workspace name:agent-alpha silent; noinitialfocus; fullscreen] env \
             HYPRPILOT_AGENT_SESSION=alpha Hyprland -c \
             '/run/user/1000/hyprpilot/sessions/alpha/hyprland.conf' > \
             '/run/user/1000/hyprpilot/sessions/alpha/hyprland.log' 2>&1"
        );

        // The command reaches `sh -c`, so a path that could close the quote is
        // refused instead of escaped.
        assert!(shell_path(Path::new("/run/user/1000/ok")).is_ok());
        assert!(shell_path(Path::new("/run/user/1000/it's")).is_err());
        assert!(
            spawn_command(
                "alpha",
                "agent-alpha",
                Path::new("/run/user/1000/it's/hyprland.conf"),
                Path::new("/run/user/1000/hyprland.log"),
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn marked_pids_cover_every_process_of_the_agent_desktop() -> Result<(), Box<dyn StdError>> {
        let root = tempfile::tempdir()?;
        let fake_process = |pid: &str, environ: &[&str]| -> Result<(), Box<dyn StdError>> {
            let dir = root.path().join(pid);
            fs::create_dir_all(&dir)?;
            let mut blob = Vec::new();
            for variable in environ {
                blob.extend_from_slice(variable.as_bytes());
                blob.push(0);
            }
            fs::write(dir.join("environ"), blob)?;
            Ok(())
        };

        fake_process("11", &["PATH=/usr/bin", "HYPRPILOT_AGENT_SESSION=alpha"])?;
        fake_process("12", &["HYPRPILOT_AGENT_SESSION=beta"])?;
        // The app the nested compositor launched inherits the marker.
        fake_process(
            "13",
            &["HYPRPILOT_AGENT_SESSION=alpha", "WAYLAND_DISPLAY=wayland-2"],
        )?;
        // Another desktop whose name merely starts the same.
        fake_process("14", &["HYPRPILOT_AGENT_SESSION=alpha2"])?;
        fs::create_dir_all(root.path().join("15"))?;
        fs::create_dir_all(root.path().join("self"))?;

        assert_eq!(marked_pids_in(root.path(), "alpha")?, vec![11, 13]);
        assert_eq!(marked_pids_in(root.path(), "alpha2")?, vec![14]);
        assert!(marked_pids_in(root.path(), "gamma")?.is_empty());
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
            size: [1920, 1080],
            shown: false,
            active_address: active.map(str::to_owned),
            instance,
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

        let live = agent_state(live_instance(), Some("0xapp"));
        assert_eq!(
            teardown_plan(&live),
            TeardownPlan {
                close: Some("0xapp"),
                instance: Some(Registered {
                    signature: "beef_1700000000",
                    pid: 4242,
                    console: "0xc0ff33",
                }),
            }
        );

        let launched_nothing = agent_state(live_instance(), None);
        let plan = teardown_plan(&launched_nothing);
        assert_eq!(plan.close, None, "no window recorded, nothing to close");
        assert!(plan.instance.is_some());
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
        let environ = |pid: &str, variables: &[&str]| -> Result<(), Box<dyn StdError>> {
            let dir = root.path().join(pid);
            fs::create_dir_all(&dir)?;
            let mut blob = Vec::new();
            for variable in variables {
                blob.extend_from_slice(variable.as_bytes());
                blob.push(0);
            }
            fs::write(dir.join("environ"), blob)?;
            Ok(())
        };
        // The live nested compositor of session `alpha` carries the marker.
        environ("4242", &["HYPRPILOT_AGENT_SESSION=alpha", "PATH=/usr/bin"])?;

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
        assert_eq!(
            user_workspace(
                &FocusedWorkspace {
                    name: "2".to_owned(),
                    monitor: "DP-3".to_owned(),
                },
                "alpha",
                &state
            )?,
            "2"
        );

        for (name, monitor) in [("agent-alpha", "hyprpilot-alpha"), ("3", "hyprpilot-alpha")] {
            let error = user_workspace(
                &FocusedWorkspace {
                    name: name.to_owned(),
                    monitor: monitor.to_owned(),
                },
                "alpha",
                &state,
            )
            .err()
            .ok_or("the agent desktop's own output was accepted as a destination")?
            .to_string();
            assert!(error.contains("hyprpilot-alpha"), "{error}");
            assert!(error.contains("waybar"), "{error}");
            assert!(error.contains("session show"), "{error}");
        }
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

        let host = snapshot(&[("DP-3", "1"), ("HDMI-A-1", "8")], Some("0xabc"));
        // The workspace a fresh headless output brings with it: empty, and not
        // one the user was looking at.
        renameable(&host, &clients, "9")?;

        let occupied = renameable(&host, &clients, "2")
            .err()
            .ok_or("a workspace holding a window was accepted")?;
        assert!(occupied.contains("0xbbb"), "{occupied}");

        let visible = renameable(&host, &clients, "8")
            .err()
            .ok_or("a workspace the user was looking at was accepted")?;
        assert!(visible.contains("visible on HDMI-A-1"), "{visible}");
        Ok(())
    }
}
