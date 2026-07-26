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
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{Error, RestoreFailure};
use crate::guard;
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
const NESTED_BINARY: &str = "Hyprland";
const NESTED_CONFIG_FILE: &str = "hyprland.conf";
const NESTED_LOG_FILE: &str = "hyprland.log";
const INSTANCE_APPEAR_TIMEOUT: Duration = Duration::from_secs(15);
const READY_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a rolled-back agent desktop gets to exit on `dispatch exit` and
/// `SIGTERM` before `SIGKILL`.
const EXIT_GRACE: Duration = Duration::from_secs(2);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);

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
            // `output remove` re-centres the user's cursor (fact §2.8).
            let cursor = hypr::cursor_pos().unwrap_or(host.cursor);
            if let Err(failure) = remove_output(&self.output) {
                return vec![failure];
            }
            if let Err(failure) = restore_cursor(cursor) {
                failures.push(failure);
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
    Ok(())
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
    Ok(environ_carries_marker(
        &Path::new("/proc").join(pid.to_string()),
        session,
    ))
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
            let _ = signal_process(pid, signal);
        }
        thread::sleep(session::POLL_INTERVAL);
    }
}

fn signal_process(pid: u32, signal: &str) -> Result<(), Error> {
    let output = Command::new("kill")
        .args(["-s", signal, "--", &pid.to_string()])
        .output()
        .map_err(|source| Error::Io {
            context: format!("running `kill -s {signal} {pid}`"),
            source,
        })?;
    if output.status.success() {
        return Ok(());
    }
    Err(Error::Tool {
        command: format!("kill -s {signal} {pid}"),
        message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

fn remove_output(output: &str) -> Result<(), RestoreFailure> {
    let failure = |actual: String| RestoreFailure {
        what: "agent output",
        expected: format!("{output} removed"),
        actual,
    };
    // An output already gone is a success, like every teardown step (§6).
    match session::find_output(output) {
        Ok(None) => return Ok(()),
        Ok(Some(_)) => {}
        Err(error) => return Err(failure(format!("looking it up failed: {error}"))),
    }
    hypr::output_remove(output).map_err(|error| failure(error.to_string()))
}

fn restore_cursor(cursor: (i32, i32)) -> Result<(), RestoreFailure> {
    let restored = guard::restore_cursor(cursor);
    let actual = hypr::cursor_pos();
    if restored.is_ok() && matches!(&actual, Ok(actual) if guard::cursor_near(*actual, cursor)) {
        return Ok(());
    }
    Err(RestoreFailure {
        what: "cursor",
        expected: format!("{cursor:?}"),
        actual: match (restored, actual) {
            (Err(error), _) => format!("restore failed: {error}"),
            (Ok(()), Ok(actual)) => format!("{actual:?}"),
            (Ok(()), Err(error)) => format!("verification failed: {error}"),
        },
    })
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

    use super::{
        AGENT_SESSION_ENV, HostSnapshot, Keymap, active_workspaces, capturable, deviation,
        dir_entries, ensure_output_absent, is_wayland_socket, keymap_of, marked_pids_in,
        nested_config, new_entries, output_is_configured, refuse_nested_marker, renameable,
        select_console, shell_path, spawn_command, wayland_sockets, workspace_occupants,
    };
    use crate::error::Error;
    use crate::hypr::{Client, Devices, Monitor};

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
