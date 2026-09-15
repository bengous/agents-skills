use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::error::Error;
use crate::{capture, hypr, isolated, keys, pointer, session};

#[derive(Parser)]
#[command(
    name = "hyprpilot",
    version,
    about = "Drive and inspect a native GUI app on a headless Hyprland output, \
             without touching the user's desktop"
)]
struct Cli {
    /// Session to act on; defaults to `$HYPRPILOT_SESSION`, else `default`
    #[arg(long, global = true, value_name = "NAME")]
    session: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
#[command(group(
    clap::ArgGroup::new("selector")
        .required(true)
        .multiple(true)
        .args(["address", "match_title", "match_class", "pid", "untracked"])
))]
struct TargetArgs {
    /// Exact Hyprland window address
    #[arg(long, value_name = "A")]
    address: Option<String>,
    /// Exact window title
    #[arg(long, value_name = "T")]
    match_title: Option<String>,
    /// Exact window class
    #[arg(long, value_name = "C")]
    match_class: Option<String>,
    /// Exact window process ID
    #[arg(long, value_name = "P")]
    pid: Option<i32>,
    /// Only consider windows not already tracked by the session
    #[arg(long)]
    untracked: bool,
    /// Poll for zero matches until this timeout, e.g. `10s`
    #[arg(long, value_name = "DURATION")]
    wait: Option<String>,
    /// What to do with a newly adopted window during teardown
    #[arg(long, value_enum, value_name = "restore|close")]
    on_teardown: Option<DispositionArg>,
}

#[derive(Subcommand)]
enum Command {
    /// Manage the driving session (one at a time)
    #[command(subcommand)]
    Session(SessionCommand),
    /// Adopt or switch to exactly one matching session window
    Target(TargetArgs),
    /// Send key chords to the session window (no focus needed)
    Key {
        /// Chords like `a`, `Down`, `Ctrl+c`, `Ctrl+Shift+Escape`
        #[arg(required = true)]
        keys: Vec<String>,
        /// Delay between chords, in milliseconds
        #[arg(long, default_value_t = 50)]
        delay_ms: u64,
        /// Temporarily focus the session window for this action
        #[arg(long)]
        focus: bool,
    },
    /// Type text character by character into the session window
    Type {
        text: String,
        /// Delay between characters, in milliseconds
        #[arg(long, default_value_t = 25)]
        delay_ms: u64,
        /// Temporarily focus the session window for this action
        #[arg(long)]
        focus: bool,
    },
    /// Click in the session window (cursor and focus are restored)
    Click {
        /// X, relative to the window unless --absolute
        #[arg(allow_negative_numbers = true)]
        x: i32,
        /// Y, relative to the window unless --absolute
        #[arg(allow_negative_numbers = true)]
        y: i32,
        #[arg(long, value_enum, default_value_t = ButtonArg::Left)]
        button: ButtonArg,
        /// Double-click: two press/release pairs ~80 ms apart
        #[arg(long)]
        double: bool,
        /// Treat X Y as global layout coordinates
        #[arg(long)]
        absolute: bool,
        /// Temporarily focus the session window for this action
        #[arg(long)]
        focus: bool,
    },
    /// Scroll wheel detents at a point in the session window (cursor and
    /// focus are restored)
    Scroll {
        /// X, relative to the window unless --absolute
        #[arg(allow_negative_numbers = true)]
        x: i32,
        /// Y, relative to the window unless --absolute
        #[arg(allow_negative_numbers = true)]
        y: i32,
        /// Vertical detents; positive = down (e.g. `--dy 5`, `--dy -3`)
        #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
        dy: i32,
        /// Horizontal detents; positive = right
        #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
        dx: i32,
        /// Treat X Y as global layout coordinates
        #[arg(long)]
        absolute: bool,
        /// Temporarily focus the session window for this action
        #[arg(long)]
        focus: bool,
    },
    /// Capture the session window (or the whole output) to a PNG
    Shot {
        /// File name (`.png` appended if missing); default: `shot-NNNN`
        name: Option<String>,
        /// Capture the entire headless output instead of the window frame
        #[arg(long)]
        full: bool,
        /// Output directory (default: the session's own `shots/` directory)
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
    },
    /// Poll window captures until the frame stabilises or changes
    Wait {
        /// Wait for two consecutive identical frames (default mode)
        #[arg(long)]
        stable: bool,
        /// Wait for a frame that differs from this reference PNG
        #[arg(long, value_name = "PNG", conflicts_with = "stable")]
        changed_from: Option<PathBuf>,
        /// e.g. `5s`, `2.5s`, `800ms`
        #[arg(long, default_value = "5s")]
        timeout: String,
    },
    /// Print session state as JSON (window, output, user focus)
    Status,
    /// List Hyprland windows as JSON
    Windows,
    /// Check the environment (hyprctl, grim, protocols, permissions)
    Doctor,
    /// End the session: close a spawned app (or return an attached window
    /// to its original workspace), remove the output
    Teardown {
        /// Kill the spawned process group instead of closing its window
        #[arg(long, conflicts_with = "close")]
        kill: bool,
        /// Close the window even if it was attached, not spawned
        #[arg(long, conflicts_with = "kill")]
        close: bool,
    },
}

#[derive(Subcommand)]
enum SessionCommand {
    /// Attach to (or launch) the app and park it on a headless output
    Start {
        /// Give this session its own nested Hyprland (agent desktop) instead
        /// of driving the user's windows
        #[arg(long)]
        isolated: bool,
        /// Command to launch if no window matches yet
        #[arg(long, value_name = "CMD")]
        app: Option<String>,
        /// Exact window title to attach to
        #[arg(long, value_name = "TITLE")]
        match_title: Option<String>,
        /// Exact window class to attach to
        #[arg(long, value_name = "CLASS")]
        match_class: Option<String>,
        /// Headless output resolution (agent desktops default to 1920x1080)
        #[arg(
            long,
            default_value = "1600x1000",
            default_value_if("isolated", "true", "1920x1080")
        )]
        size: String,
    },
    /// Resize the session output without recreating the session
    Resize {
        /// New headless output resolution
        #[arg(value_name = "WxH")]
        size: String,
    },
    /// Agent desktops only: put the console window on the workspace you are
    /// looking at, floating
    Show,
    /// Agent desktops only: send the console window back to `agent-<session>`
    Hide,
}

#[derive(Clone, Copy, ValueEnum)]
enum ButtonArg {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, ValueEnum)]
enum DispositionArg {
    Restore,
    Close,
}

impl From<DispositionArg> for session::Disposition {
    fn from(disposition: DispositionArg) -> Self {
        match disposition {
            DispositionArg::Restore => Self::Restore,
            DispositionArg::Close => Self::Close,
        }
    }
}

impl From<ButtonArg> for pointer::MouseButton {
    fn from(button: ButtonArg) -> Self {
        match button {
            ButtonArg::Left => Self::Left,
            ButtonArg::Right => Self::Right,
            ButtonArg::Middle => Self::Middle,
        }
    }
}

pub fn run() -> Result<String, Error> {
    let cli = Cli::parse();
    // The precondition of every command, not only of a start: inside an agent
    // desktop `hyprctl` answers for the nested compositor — an output created
    // there stays 0x0 and captures of it are silently blank — and every process
    // around, this one's own shell included, carries that desktop's session
    // marker.
    isolated::refuse_when_nested()?;
    let session_name = session::resolve_name(cli.session.as_deref())?;
    let name = session_name.as_str();
    match cli.command {
        Command::Session(SessionCommand::Start {
            isolated,
            app,
            match_title,
            match_class,
            size,
        }) => session::start(
            name,
            isolated,
            app.as_deref(),
            match_title.as_deref(),
            match_class.as_deref(),
            &size,
        ),
        Command::Session(SessionCommand::Resize { size }) => session::resize(name, &size),
        Command::Session(SessionCommand::Show) => session::show(name),
        Command::Session(SessionCommand::Hide) => session::hide(name),
        Command::Target(TargetArgs {
            address,
            match_title,
            match_class,
            pid,
            untracked,
            wait,
            on_teardown,
        }) => {
            let wait = wait.as_deref().map(capture::parse_timeout).transpose()?;
            let criteria = session::Criteria {
                address: address.as_deref(),
                title: match_title.as_deref(),
                class: match_class.as_deref(),
                pid,
            };
            session::target(
                name,
                &criteria,
                untracked,
                wait,
                on_teardown.map(Into::into),
            )
        }
        Command::Key {
            keys,
            delay_ms,
            focus,
        } => crate::keys::send_keys(name, &keys, delay_ms, focus),
        Command::Type {
            text,
            delay_ms,
            focus,
        } => keys::type_text(name, &text, delay_ms, focus),
        Command::Click {
            x,
            y,
            button,
            double,
            absolute,
            focus,
        } => pointer::click(name, x, y, button.into(), double, absolute, focus),
        Command::Scroll {
            x,
            y,
            dy,
            dx,
            absolute,
            focus,
        } => pointer::scroll(name, x, y, dx, dy, absolute, focus),
        Command::Shot {
            name: file_name,
            full,
            out,
        } => capture::shot(name, file_name.as_deref(), full, out.as_deref()),
        Command::Wait {
            changed_from,
            timeout,
            ..
        } => {
            let mode =
                changed_from.map_or(capture::WaitMode::Stable, capture::WaitMode::ChangedFrom);
            let timeout = capture::parse_timeout(&timeout)?;
            capture::wait(name, &mode, timeout)
        }
        Command::Status => status(name),
        Command::Windows => windows(name),
        Command::Doctor => doctor(),
        Command::Teardown { kill, close } => session::teardown(name, kill, close),
    }
}

/// Whose windows `windows` is listing, and what it knows about them.
enum WindowSession {
    Absent,
    Shared(session::Shared),
    /// An agent desktop records one window at a time instead of a list of
    /// adopted ones, so the recorded window is both the tracked and the active
    /// one.
    Agent(Option<String>),
    Unknown,
}

#[derive(Serialize)]
struct WindowInfo<'a> {
    address: &'a str,
    class: &'a str,
    initial_class: &'a str,
    title: &'a str,
    initial_title: &'a str,
    pid: i32,
    workspace: &'a str,
    at: [i32; 2],
    size: [i32; 2],
    floating: bool,
    focused: bool,
    monitor: i64,
    tracked: Option<bool>,
    active: Option<bool>,
}

fn serialize_windows(
    clients: &[hypr::Client],
    focused: Option<&hypr::Client>,
    session: &WindowSession,
) -> Result<String, Error> {
    let focused_address = focused.map(|client| client.address.as_str());
    let windows = clients
        .iter()
        .map(|client| {
            let (tracked, active) = match session {
                WindowSession::Absent => (Some(false), Some(false)),
                WindowSession::Shared(session) => (
                    Some(
                        session
                            .windows
                            .iter()
                            .any(|window| window.address == client.address),
                    ),
                    Some(session.active_address == client.address),
                ),
                WindowSession::Agent(recorded) => {
                    let recorded = recorded.as_deref() == Some(client.address.as_str());
                    (Some(recorded), Some(recorded))
                }
                WindowSession::Unknown => (None, None),
            };
            WindowInfo {
                address: &client.address,
                class: &client.class,
                initial_class: &client.initial_class,
                title: &client.title,
                initial_title: &client.initial_title,
                pid: client.pid,
                workspace: &client.workspace.name,
                at: client.at,
                size: client.size,
                floating: client.floating,
                focused: focused_address == Some(client.address.as_str()),
                monitor: client.monitor,
                tracked,
                active,
            }
        })
        .collect::<Vec<_>>();

    serde_json::to_string_pretty(&windows).map_err(|source| Error::Json {
        context: "serializing windows".to_owned(),
        source,
    })
}

fn windows(name: &str) -> Result<String, Error> {
    // The session is resolved before the first compositor read: an isolated
    // session lists the clients of its own instance, not the user's, so
    // querying the host here would print a plausible, wrong answer.
    let session = match session::load(name) {
        Ok(session) => match session.state {
            session::ModeState::Shared(shared) => WindowSession::Shared(shared),
            session::ModeState::Isolated(isolated) => return agent_windows(name, &isolated),
        },
        Err(Error::NoSession) => WindowSession::Absent,
        Err(
            error @ (Error::Json { .. }
            | Error::CorruptSession { .. }
            | Error::UnsupportedSessionVersion { .. }),
        ) => {
            let _ = writeln!(
                std::io::stderr(),
                "hyprpilot: warning: cannot read session state ({error}); \
                 tracked and active are null"
            );
            WindowSession::Unknown
        }
        Err(error) => return Err(error),
    };
    let clients = hypr::clients()?;
    let focused = hypr::active_window()?;
    serialize_windows(&clients, focused.as_ref(), &session)
}

/// The clients of the instance, with the same fields and the same annotations
/// as on the host. `focused` comes from the instance's own active window, so it
/// answers about the agent desktop's seat and not the user's.
fn agent_windows(name: &str, isolated: &session::Isolated) -> Result<String, Error> {
    let ctl = isolated::live_instance(name, isolated)?.ctl();
    let clients = hypr::clients_on(ctl)?;
    let focused = hypr::active_window_on(ctl)?;
    serialize_windows(
        &clients,
        focused.as_ref(),
        &WindowSession::Agent(isolated.active_address.clone()),
    )
}

fn status(name: &str) -> Result<String, Error> {
    let session = session::load(name)?;
    // Routed before the first compositor read: an agent desktop's geometry is
    // read inside its own instance, never on the host.
    match &session.state {
        session::ModeState::Shared(shared) => shared_status(&session, shared),
        session::ModeState::Isolated(isolated) => agent_status(name, &session, isolated),
    }
}

/// Mode, session, instance identity, show/hide state, the configured and
/// effective size of the agent desktop, and the geometry of its window as the
/// *instance* reports it. The keys are a contract, asserted by a test.
fn agent_status(
    name: &str,
    session: &session::Session,
    isolated: &session::Isolated,
) -> Result<String, Error> {
    let instance = isolated::live_instance(name, isolated)?;
    let ctl = instance.ctl();
    let host_output = session::find_output(&isolated.output)?;
    let outputs = hypr::monitors_on(ctl)?;
    let clients = hypr::clients_on(ctl)?;
    let window = isolated
        .active_address
        .as_deref()
        .and_then(|address| clients.iter().find(|client| client.address == address));
    serialize_status(&agent_status_value(
        session,
        isolated,
        instance,
        // The nested config gives an agent desktop exactly one output.
        outputs.first(),
        window,
        host_output.as_ref(),
    ))
}

fn agent_status_value(
    session: &session::Session,
    isolated: &session::Isolated,
    instance: isolated::LiveInstance<'_>,
    agent_output: Option<&hypr::Monitor>,
    window: Option<&hypr::Client>,
    host_output: Option<&hypr::Monitor>,
) -> serde_json::Value {
    let effective_size = agent_output.map(|monitor| [monitor.width, monitor.height]);
    let size_mismatch = effective_size.map(|size| sizes_mismatch(isolated.size, size));

    serde_json::json!({
        "schema_version": session.schema_version,
        "session": session.name,
        "mode": session.mode(),
        "workspace": isolated.workspace,
        "shown": isolated.shown,
        "instance": {
            "signature": instance.signature,
            "wayland_display": instance.wayland_display,
            "pid": instance.pid,
            "console_address": instance.console,
        },
        "active_address": isolated.active_address,
        "configured_size": isolated.size,
        "effective_size": effective_size,
        "size_mismatch": size_mismatch,
        "window": window.map(|window| serde_json::json!({
            "address": window.address,
            "title": window.title,
            "class": window.class,
            "at": window.at,
            "size": window.size,
            "workspace": window.workspace.name,
            "floating": window.floating,
            "monitor": window.monitor,
            "pid": window.pid,
        })),
        "agent_output": agent_output.map(|monitor| serde_json::json!({
            "name": monitor.name,
            "width": monitor.width,
            "height": monitor.height,
            "scale": monitor.scale,
            "active_workspace": monitor.active_workspace.name,
            "special_workspace": monitor.special_workspace,
        })),
        // The host side of the frame-callback invariant: captures only work
        // while `workspace` is the active one on this output.
        "output": host_output.map(|monitor| serde_json::json!({
            "id": monitor.id,
            "name": monitor.name,
            "x": monitor.x,
            "y": monitor.y,
            "width": monitor.width,
            "height": monitor.height,
            "active_workspace": monitor.active_workspace.name,
            "special_workspace": monitor.special_workspace,
        })),
    })
}

fn serialize_status(value: &serde_json::Value) -> Result<String, Error> {
    serde_json::to_string_pretty(value).map_err(|source| Error::Json {
        context: "serializing status".to_owned(),
        source,
    })
}

fn shared_status(session: &session::Session, state: &session::Shared) -> Result<String, Error> {
    let mode = session.mode();
    let clients = hypr::clients()?;
    let window = clients
        .iter()
        .find(|client| client.address == state.active_address)
        .ok_or_else(|| Error::WindowGone(state.active_address.clone()))?;
    let output = session::find_output(&state.output)?;
    let active = hypr::active_window()?;
    let parked_windows = state
        .windows
        .iter()
        .filter(|tracked| {
            clients.iter().any(|client| {
                client.address == tracked.address
                    && client.workspace.name == state.parking_workspace
            })
        })
        .map(|tracked| tracked.address.as_str())
        .collect::<Vec<_>>();
    let effective_size = output
        .as_ref()
        .map(|monitor| [monitor.width, monitor.height]);
    let size_mismatch = effective_size.map(|size| sizes_mismatch(state.size, size));

    let value = serde_json::json!({
        "schema_version": session.schema_version,
        "session": session.name,
        "mode": mode,
        "windows": &state.windows,
        "active_address": &state.active_address,
        "parked_windows": parked_windows,
        "configured_size": state.size,
        "effective_size": effective_size,
        "size_mismatch": size_mismatch,
        "window": {
            "address": window.address,
            "title": window.title,
            "class": window.class,
            "at": window.at,
            "size": window.size,
            "workspace": window.workspace.name,
            "floating": window.floating,
            "monitor": window.monitor,
            "pid": window.pid,
        },
        "output": output.map(|monitor| serde_json::json!({
            "id": monitor.id,
            "name": monitor.name,
            "x": monitor.x,
            "y": monitor.y,
            "width": monitor.width,
            "height": monitor.height,
            "active_workspace": monitor.active_workspace.name,
            "special_workspace": monitor.special_workspace,
        })),
        "user_active_window": active.map(|window| serde_json::json!({
            "address": window.address,
            "title": window.title,
        })),
        "initial_user_focus": &state.initial_user_focus,
        "attached": state.spawned.is_none(),
        "spawned_pid": state.spawned.map(|group| group.pid),
    });
    serialize_status(&value)
}

fn sizes_mismatch(configured: [u32; 2], effective: [f64; 2]) -> bool {
    effective
        .into_iter()
        .zip(configured)
        .any(|(actual, expected)| {
            actual.total_cmp(&f64::from(expected)) != std::cmp::Ordering::Equal
        })
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(".doctor-probe");
    fs::create_dir_all(dir).is_ok()
        && fs::write(&probe, b"probe").is_ok()
        && fs::remove_file(&probe).is_ok()
}

/// Accumulated `doctor` output. Only `FAIL` lines decide the exit status: a
/// `warn` is something to know about, not a broken environment.
#[derive(Default)]
struct Report {
    lines: Vec<String>,
    failures: usize,
}

impl Report {
    fn check(&mut self, ok: bool, success: &str, failure: &str) {
        if ok {
            self.ok(success);
        } else {
            self.fail(failure);
        }
    }

    fn ok(&mut self, line: &str) {
        self.lines.push(format!("ok    {line}"));
    }

    fn warn(&mut self, line: &str) {
        self.lines.push(format!("warn  {line}"));
    }

    fn info(&mut self, line: &str) {
        self.lines.push(format!("info  {line}"));
    }

    fn fail(&mut self, line: &str) {
        self.failures += 1;
        self.lines.push(format!("FAIL  {line}"));
    }

    fn finish(self) -> Result<String, Error> {
        let report = self.lines.join("\n");
        if self.failures == 0 {
            Ok(report)
        } else {
            Err(Error::DoctorFailed {
                report,
                failures: self.failures,
            })
        }
    }
}

/// The Hyprland version the agent desktop recipe was validated on, on
/// 2026-07-24. Another version is a warning, not a failure: the recipe may still
/// hold, but the console window class and the frame-callback behaviour it relies
/// on were only ever observed on this one.
const TESTED_HYPRLAND: &str = "0.56";

/// What isolated mode needs from the environment. Read once, so the checks below
/// stay pure — `doctor` requires no session, and neither does asserting it.
struct AgentProbe {
    binary: bool,
    /// First line of `hyprctl version`, or `None` when it could not be read.
    version: Option<String>,
    sessions_dir: PathBuf,
    sessions_writable: bool,
    /// `$XDG_RUNTIME_DIR/hypr`, where every compositor keeps its instance
    /// directory and its `.socket.sock`: a start discovers its nested
    /// compositor by diffing this directory and every `hyprctl -i` reaches it
    /// through the socket inside.
    instances_dir: PathBuf,
    instances_listable: bool,
    /// Whether the *host's* own instance socket is in there, i.e. whether that
    /// layout is the one this machine actually uses. `None` when the signature is
    /// unknown, which the check above already reports.
    host_socket: Option<bool>,
}

/// The version token of a `hyprctl version` line, e.g. `0.56.0` out of
/// `Hyprland 0.56.0 built from branch …`.
fn version_number(line: &str) -> Option<&str> {
    line.split_whitespace()
        .map(|token| token.trim_start_matches('v'))
        .find(|token| token.contains('.') && token.starts_with(|c: char| c.is_ascii_digit()))
}

fn agent_checks(probe: &AgentProbe, report: &mut Report) {
    let binary = isolated::NESTED_BINARY;
    report.check(
        probe.binary,
        &format!("{binary} found on PATH — an agent desktop is a nested {binary}"),
        &format!(
            "{binary} not found on PATH — `session start --isolated` cannot spawn an agent desktop"
        ),
    );

    match probe.version.as_deref().and_then(version_number) {
        Some(version) if version.starts_with(TESTED_HYPRLAND) => report.ok(&format!(
            "Hyprland {version} is the version agent desktops were validated on"
        )),
        Some(version) => report.warn(&format!(
            "Hyprland {version}: agent desktops were validated on {TESTED_HYPRLAND} — the nested \
             spawn, the console window class and the frame-callback behaviour captures depend on \
             may differ"
        )),
        None => report.warn(&format!(
            "no version number in `hyprctl version` — agent desktops were validated on Hyprland \
             {TESTED_HYPRLAND}"
        )),
    }

    let dir = probe.sessions_dir.display();
    report.check(
        probe.sessions_writable,
        &format!("{dir} is writable — sessions can be claimed"),
        &format!("{dir} is not writable — no session can be claimed there"),
    );

    let instances = probe.instances_dir.display();
    report.check(
        probe.instances_listable,
        &format!(
            "{instances} is listable — a start finds its nested compositor by diffing it, and \
             `hyprctl -i` talks to the socket inside"
        ),
        &format!(
            "{instances} cannot be listed — a start cannot discover the instance it spawns, and no \
             command could address it"
        ),
    );
    match probe.host_socket {
        Some(true) => report.ok(&format!(
            "this session's own instance socket is in {instances} — the layout an agent desktop is \
             addressed through"
        )),
        Some(false) => report.fail(&format!(
            "this session's own instance socket is missing from {instances} — an agent desktop is \
             addressed through that layout, so a nested compositor could not be reached either"
        )),
        None => {}
    }
}

/// Whether the running compositor's own `.socket.sock` sits where an agent
/// desktop's would, i.e. whether `hyprctl -i <signature>` can reach one at all.
/// `None` when no signature is exported — the check for that stands on its own.
fn host_instance_socket(instances_dir: &Path) -> Option<bool> {
    let signature = env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(instances_dir.join(signature).join(".socket.sock").exists())
}

/// Hyprland retracts no `keyword` while it runs (hyprwm/Hyprland#5691), so a
/// session that ends still leaves its `monitor` and `workspace` rules behind.
/// They are harmless and they accumulate; `doctor` names them so the leak is
/// visible instead of silent, with the one command that clears it.
fn report_host_leaks(report: &mut Report) {
    let leaks = session::live_host_leaks();
    if leaks.is_empty() {
        return;
    }
    report.info(&format!(
        "{} host rule(s) posed by hyprpilot sessions cannot be retracted while Hyprland runs and \
         stay until `hyprctl reload`: {}",
        leaks.len(),
        leaks.join(", ")
    ));
}

fn doctor() -> Result<String, Error> {
    let mut report = Report::default();

    let version = match hypr::version() {
        Ok(version) => {
            let first_line = version.lines().next().unwrap_or("unknown").to_owned();
            report.ok(&first_line);
            Some(first_line)
        }
        Err(error) => {
            report.fail(&format!("hyprctl: {error}"));
            None
        }
    };

    report.check(
        env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
        "HYPRLAND_INSTANCE_SIGNATURE is set",
        "HYPRLAND_INSTANCE_SIGNATURE is not set — not inside a Hyprland session?",
    );

    report.check(
        session::binary_on_path("grim"),
        "grim found on PATH",
        "grim not found on PATH — install grim for captures",
    );

    match session::runtime_dir() {
        Ok(dir) => report.check(
            writable(&dir),
            &format!("{} is writable", dir.display()),
            &format!("{} is not writable", dir.display()),
        ),
        Err(error) => report.fail(&error.to_string()),
    }

    match (session::sessions_dir(), isolated::instances_dir()) {
        (Ok(sessions_dir), Ok(instances_dir)) => agent_checks(
            &AgentProbe {
                binary: isolated::nested_binary_present(),
                version,
                sessions_writable: writable(&sessions_dir),
                sessions_dir,
                instances_listable: fs::read_dir(&instances_dir).is_ok(),
                host_socket: host_instance_socket(&instances_dir),
                instances_dir,
            },
            &mut report,
        ),
        (Err(error), _) | (_, Err(error)) => report.fail(&error.to_string()),
    }

    report_host_leaks(&mut report);

    match pointer::probe_virtual_pointer() {
        Ok(present) => report.check(
            present,
            "zwlr_virtual_pointer_manager_v1 is available",
            "compositor does not expose zwlr_virtual_pointer_manager_v1 — `click` will not work",
        ),
        Err(error) => report.fail(&error.to_string()),
    }
    report.info(
        "the virtual pointer probe above asked the compositor of $WAYLAND_DISPLAY, which is the \
         host: an agent desktop's own nested compositor is only probed when `click` or `scroll` run \
         against that session",
    );

    match hypr::devices() {
        Ok(devices) => {
            let (layout, keymap) = devices.keyboards.iter().find(|k| k.main).map_or_else(
                || ("unknown".to_owned(), "unknown".to_owned()),
                |k| (k.layout.clone(), k.active_keymap.clone()),
            );
            report.info(&format!(
                "main keyboard layout `{layout}` (active keymap `{keymap}`) — `type` maps \
                 characters through US shift pairs; accented characters need a keymap exposing \
                 them (e.g. fr)"
            ));
        }
        Err(error) => report.fail(&format!("hyprctl devices: {error}")),
    }

    report.finish()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error as StdError;

    use clap::Parser;

    use super::{
        AgentProbe, Cli, Command, Report, SessionCommand, TESTED_HYPRLAND, WindowSession,
        agent_checks, agent_status_value, serialize_windows, sizes_mismatch, version_number,
    };
    use crate::error::Error;
    use crate::hypr::{Client, Monitor};
    use crate::isolated::LiveInstance;
    use crate::session::{
        self, Disposition, Instance, Isolated, ModeState, Session, Shared, TrackedWindow,
    };
    use std::path::PathBuf;

    const CLIENTS_JSON: &str = include_str!("../fixtures/clients.json");
    const INSTANCE_CLIENTS_JSON: &str = include_str!("../fixtures/clients-ambiguous.json");
    const MONITORS_JSON: &str = include_str!("../fixtures/monitors.json");

    fn clients() -> Result<Vec<Client>, serde_json::Error> {
        serde_json::from_str(CLIENTS_JSON)
    }

    fn instance_clients() -> Result<Vec<Client>, serde_json::Error> {
        serde_json::from_str(INSTANCE_CLIENTS_JSON)
    }

    fn monitors() -> Result<Vec<Monitor>, serde_json::Error> {
        serde_json::from_str(MONITORS_JSON)
    }

    fn agent_state(active: Option<&str>) -> Isolated {
        Isolated {
            output: "hyprpilot-alpha".to_owned(),
            workspace: "agent-alpha".to_owned(),
            instance_nonce: "4242-1700000000000000000".to_owned(),
            size: [1920, 1080],
            shown: false,
            active_address: active.map(str::to_owned),
            instance: Instance::Live {
                signature: "beef_1700000000".to_owned(),
                wayland_display: "wayland-3".to_owned(),
                pid: 4242,
                console_address: "0xc0ff33".to_owned(),
            },
            host: Vec::new(),
        }
    }

    fn agent_session(isolated: &Isolated) -> Session {
        Session {
            schema_version: session::SCHEMA_VERSION,
            name: "alpha".to_owned(),
            state: ModeState::Isolated(isolated.clone()),
        }
    }

    fn agent_instance() -> LiveInstance<'static> {
        LiveInstance {
            signature: "beef_1700000000",
            wayland_display: "wayland-3",
            pid: 4242,
            console: "0xc0ff33",
        }
    }

    fn tracked(client: &Client) -> TrackedWindow {
        TrackedWindow {
            address: client.address.clone(),
            stable_id: client.stable_id.clone(),
            title_at_adoption: client.title.clone(),
            origin_workspace: client.workspace.name.clone(),
            origin_at: client.at,
            origin_size: client.size,
            origin_floating: client.floating,
            teardown: Disposition::Restore,
        }
    }

    fn valid_session(clients: &[Client]) -> Shared {
        Shared {
            output: "hyprpilot".to_owned(),
            active_workspace: "hyprpilot".to_owned(),
            parking_workspace: "special:hyprpilot-parked".to_owned(),
            size: [1600, 1000],
            spawned: None,
            initial_user_focus: None,
            primary_address: clients[0].address.clone(),
            active_address: clients[1].address.clone(),
            windows: vec![tracked(&clients[0]), tracked(&clients[1])],
            host: Vec::new(),
        }
    }

    #[test]
    fn teardown_kill_and_close_conflict() {
        assert!(Cli::try_parse_from(["hyprpilot", "teardown", "--kill", "--close"]).is_err());
    }

    #[test]
    fn target_requires_at_least_one_selector() {
        assert!(Cli::try_parse_from(["hyprpilot", "target"]).is_err());
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "target", "--untracked"]),
            Ok(Cli {
                command: Command::Target(_),
                ..
            })
        ));
    }

    #[test]
    fn session_resize_parses_size_with_existing_parser() -> Result<(), Box<dyn StdError>> {
        let Cli {
            command: Command::Session(SessionCommand::Resize { size }),
            ..
        } = Cli::try_parse_from(["hyprpilot", "session", "resize", "1200x800"])?
        else {
            return Err("session resize did not parse".into());
        };

        assert_eq!(session::parse_size(&size)?, (1200, 800));
        Ok(())
    }

    #[test]
    fn session_flag_parses_before_and_after_the_subcommand() -> Result<(), Box<dyn StdError>> {
        for args in [
            ["hyprpilot", "--session", "alpha", "shot"],
            ["hyprpilot", "shot", "--session", "alpha"],
        ] {
            let cli = Cli::try_parse_from(args)?;
            assert_eq!(cli.session.as_deref(), Some("alpha"), "{args:?}");
        }
        assert_eq!(Cli::try_parse_from(["hyprpilot", "shot"])?.session, None);
        Ok(())
    }

    fn started_size(flags: &[&str]) -> Result<(u32, u32), Box<dyn StdError>> {
        let argv = ["hyprpilot", "session", "start"]
            .into_iter()
            .chain(flags.iter().copied());
        let Cli {
            command: Command::Session(SessionCommand::Start { size, .. }),
            ..
        } = Cli::try_parse_from(argv)?
        else {
            return Err("session start did not parse".into());
        };
        Ok(session::parse_size(&size)?)
    }

    #[test]
    fn session_start_parses_isolated_flag() -> Result<(), Box<dyn StdError>> {
        let Cli {
            command: Command::Session(SessionCommand::Start { isolated, size, .. }),
            session,
        } = Cli::try_parse_from([
            "hyprpilot",
            "--session",
            "agent-1",
            "session",
            "start",
            "--isolated",
            "--app",
            "my-app",
            "--match-title",
            "My App",
        ])?
        else {
            return Err("session start --isolated did not parse".into());
        };

        assert!(isolated);
        assert_eq!(session.as_deref(), Some("agent-1"));
        // An agent desktop defaults to 1920x1080, the shared output to
        // 1600x1000, and an explicit `--size` wins over both.
        assert_eq!(session::parse_size(&size)?, (1920, 1080));
        assert_eq!(started_size(&["--match-title", "T"])?, (1600, 1000));
        assert_eq!(
            started_size(&["--isolated", "--match-title", "T", "--size", "800x600"])?,
            (800, 600)
        );
        assert!(
            !matches!(
                Cli::try_parse_from(["hyprpilot", "session", "start", "--match-title", "T"]),
                Ok(Cli {
                    command: Command::Session(SessionCommand::Start { isolated: true, .. }),
                    ..
                })
            ),
            "shared start must not be isolated by default"
        );
        Ok(())
    }

    #[test]
    fn status_detects_configured_effective_size_mismatch() {
        assert!(sizes_mismatch([1600, 1000], [1200.0, 800.0]));
    }

    #[test]
    fn target_accepts_combined_selectors_wait_and_disposition() {
        assert!(matches!(
            Cli::try_parse_from([
                "hyprpilot",
                "target",
                "--address",
                "0xabc",
                "--match-title",
                "App",
                "--match-class",
                "app",
                "--pid",
                "42",
                "--untracked",
                "--wait",
                "5s",
                "--on-teardown",
                "close",
            ]),
            Ok(Cli {
                command: Command::Target(_),
                ..
            })
        ));
    }

    #[test]
    fn action_commands_parse_focus_flag() {
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "key", "--focus", "Return"]),
            Ok(Cli {
                command: Command::Key { focus: true, .. },
                ..
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "type", "--focus", "text"]),
            Ok(Cli {
                command: Command::Type { focus: true, .. },
                ..
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "click", "--focus", "20", "20"]),
            Ok(Cli {
                command: Command::Click { focus: true, .. },
                ..
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "scroll", "--focus", "20", "20", "--dy", "1",]),
            Ok(Cli {
                command: Command::Scroll { focus: true, .. },
                ..
            })
        ));
    }

    #[test]
    fn windows_serializes_exact_fields_and_calculates_focus() -> Result<(), Box<dyn StdError>> {
        let clients = clients()?;
        let raw = serialize_windows(&clients, Some(&clients[1]), &WindowSession::Absent)?;
        let rows: Vec<serde_json::Value> = serde_json::from_str(&raw)?;
        let keys = rows[0]
            .as_object()
            .ok_or("window row is not an object")?
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected = [
            "address",
            "class",
            "initial_class",
            "title",
            "initial_title",
            "pid",
            "workspace",
            "at",
            "size",
            "floating",
            "focused",
            "monitor",
            "tracked",
            "active",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();

        assert_eq!(keys, expected);
        assert_eq!(
            rows.iter()
                .filter(|row| row["focused"] == serde_json::Value::Bool(true))
                .map(|row| row["address"].as_str())
                .collect::<Vec<_>>(),
            vec![Some(clients[1].address.as_str())]
        );
        Ok(())
    }

    #[test]
    fn windows_without_session_marks_every_client_untracked_and_inactive()
    -> Result<(), Box<dyn StdError>> {
        let clients = clients()?;
        let raw = serialize_windows(&clients, None, &WindowSession::Absent)?;
        let rows: Vec<serde_json::Value> = serde_json::from_str(&raw)?;

        assert!(
            rows.iter()
                .all(|row| row["tracked"] == false && row["active"] == false)
        );
        Ok(())
    }

    #[test]
    fn windows_with_valid_session_marks_tracked_and_active_addresses()
    -> Result<(), Box<dyn StdError>> {
        let clients = clients()?;
        let session = WindowSession::Shared(valid_session(&clients));
        let raw = serialize_windows(&clients, None, &session)?;
        let rows: Vec<serde_json::Value> = serde_json::from_str(&raw)?;

        assert_eq!(
            rows.iter()
                .map(|row| (row["tracked"].as_bool(), row["active"].as_bool()))
                .collect::<Vec<_>>(),
            vec![
                (Some(true), Some(false)),
                (Some(true), Some(true)),
                (Some(false), Some(false)),
            ]
        );
        Ok(())
    }

    #[test]
    fn windows_with_corrupt_session_marks_annotations_unknown() -> Result<(), Box<dyn StdError>> {
        let clients = clients()?;
        let raw = serialize_windows(&clients, None, &WindowSession::Unknown)?;
        let rows: Vec<serde_json::Value> = serde_json::from_str(&raw)?;

        assert!(
            rows.iter()
                .all(|row| row["tracked"].is_null() && row["active"].is_null())
        );
        Ok(())
    }

    #[test]
    fn windows_without_clients_is_an_empty_json_array() -> Result<(), Box<dyn StdError>> {
        assert_eq!(serialize_windows(&[], None, &WindowSession::Absent)?, "[]");
        Ok(())
    }

    #[test]
    fn agent_windows_keep_the_shared_fields_and_annotate_the_recorded_window()
    -> Result<(), Box<dyn StdError>> {
        let clients = instance_clients()?;
        let host = clients_rows(&clients, Some(&clients[1]), &WindowSession::Absent)?;
        let agent = clients_rows(
            &clients,
            // `focused` comes from the *instance's* active window.
            Some(&clients[3]),
            &WindowSession::Agent(Some(clients[1].address.clone())),
        )?;

        assert_eq!(keys_of(&host[0])?, keys_of(&agent[0])?);
        assert_eq!(
            agent
                .iter()
                .map(|row| (
                    row["address"].as_str(),
                    row["tracked"].as_bool(),
                    row["active"].as_bool(),
                    row["focused"].as_bool(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (Some("0xaaa"), Some(false), Some(false), Some(false)),
                // The recorded window is the tracked one and the active one.
                (Some("0xbbb"), Some(true), Some(true), Some(false)),
                (Some("0xccc"), Some(false), Some(false), Some(false)),
                (Some("0xddd"), Some(false), Some(false), Some(true)),
            ]
        );

        // A start that recorded nothing yet annotates every client as untracked.
        let empty = clients_rows(&clients, None, &WindowSession::Agent(None))?;
        assert!(
            empty
                .iter()
                .all(|row| row["tracked"] == false && row["active"] == false),
            "{empty:?}"
        );
        Ok(())
    }

    fn clients_rows(
        clients: &[Client],
        focused: Option<&Client>,
        session: &WindowSession,
    ) -> Result<Vec<serde_json::Value>, Box<dyn StdError>> {
        Ok(serde_json::from_str(&serialize_windows(
            clients, focused, session,
        )?)?)
    }

    fn keys_of(row: &serde_json::Value) -> Result<BTreeSet<&str>, Box<dyn StdError>> {
        Ok(row
            .as_object()
            .ok_or("window row is not an object")?
            .keys()
            .map(String::as_str)
            .collect())
    }

    #[test]
    fn session_show_and_hide_parse_with_the_session_flag_on_either_side()
    -> Result<(), Box<dyn StdError>> {
        for args in [
            ["hyprpilot", "--session", "alpha", "session", "show"],
            ["hyprpilot", "session", "show", "--session", "alpha"],
        ] {
            let cli = Cli::try_parse_from(args)?;
            assert_eq!(cli.session.as_deref(), Some("alpha"), "{args:?}");
            assert!(
                matches!(cli.command, Command::Session(SessionCommand::Show)),
                "{args:?}"
            );
        }
        for args in [
            ["hyprpilot", "--session", "beta", "session", "hide"],
            ["hyprpilot", "session", "hide", "--session", "beta"],
        ] {
            let cli = Cli::try_parse_from(args)?;
            assert_eq!(cli.session.as_deref(), Some("beta"), "{args:?}");
            assert!(
                matches!(cli.command, Command::Session(SessionCommand::Hide)),
                "{args:?}"
            );
        }
        // Neither takes an argument: a stray one is a parse error, not a session
        // name.
        assert!(Cli::try_parse_from(["hyprpilot", "session", "show", "alpha"]).is_err());
        assert!(Cli::try_parse_from(["hyprpilot", "session", "hide", "alpha"]).is_err());
        Ok(())
    }

    #[test]
    fn isolated_status_holds_exactly_these_keys() -> Result<(), Box<dyn StdError>> {
        let isolated = agent_state(Some("0xbbb"));
        let session = agent_session(&isolated);
        let monitors = monitors()?;
        let clients = instance_clients()?;
        let window = clients
            .iter()
            .find(|client| client.address == "0xbbb")
            .ok_or("instance fixture lost 0xbbb")?;

        let value = agent_status_value(
            &session,
            &isolated,
            agent_instance(),
            Some(&monitors[1]),
            Some(window),
            Some(&monitors[1]),
        );
        let object = value.as_object().ok_or("status is not an object")?;

        assert_eq!(
            object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            [
                "schema_version",
                "session",
                "mode",
                "workspace",
                "shown",
                "instance",
                "active_address",
                "configured_size",
                "effective_size",
                "size_mismatch",
                "window",
                "agent_output",
                "output",
            ]
            .into_iter()
            .collect::<BTreeSet<_>>()
        );
        assert_eq!(
            keys_of(&value["instance"])?,
            ["signature", "wayland_display", "pid", "console_address"]
                .into_iter()
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(
            keys_of(&value["window"])?,
            [
                "address",
                "title",
                "class",
                "at",
                "size",
                "workspace",
                "floating",
                "monitor",
                "pid"
            ]
            .into_iter()
            .collect::<BTreeSet<_>>()
        );

        assert_eq!(value["mode"], "isolated");
        assert_eq!(value["session"], "alpha");
        assert_eq!(value["workspace"], "agent-alpha");
        assert_eq!(value["shown"], false);
        assert_eq!(value["instance"]["signature"], "beef_1700000000");
        assert_eq!(value["instance"]["wayland_display"], "wayland-3");
        assert_eq!(value["instance"]["pid"], 4242);
        assert_eq!(value["instance"]["console_address"], "0xc0ff33");
        assert_eq!(value["active_address"], "0xbbb");
        // The geometry is the instance's, and the fixture headless output is
        // 1600x1000 against a configured 1920x1080: the drift is reported.
        assert_eq!(value["window"]["at"], serde_json::json!([30, 40]));
        assert_eq!(value["configured_size"], serde_json::json!([1920, 1080]));
        assert_eq!(value["effective_size"], serde_json::json!([1600.0, 1000.0]));
        assert_eq!(value["size_mismatch"], true);
        // The host side of the frame-callback invariant is readable from
        // `output` alone.
        assert_eq!(value["output"]["name"], "headless-ci");
        assert_eq!(value["output"]["active_workspace"], "proto");
        Ok(())
    }

    #[test]
    fn isolated_status_keeps_its_keys_when_no_window_is_recorded() {
        let isolated = agent_state(None);
        let value = agent_status_value(
            &agent_session(&isolated),
            &isolated,
            agent_instance(),
            None,
            None,
            None,
        );

        assert!(value["active_address"].is_null());
        assert!(value["window"].is_null());
        assert!(value["agent_output"].is_null());
        assert!(value["output"].is_null());
        assert!(value["effective_size"].is_null());
        assert!(value["size_mismatch"].is_null());
        assert_eq!(value["configured_size"], serde_json::json!([1920, 1080]));
    }

    #[test]
    fn doctor_checks_the_isolated_mode_without_a_session() -> Result<(), Box<dyn StdError>> {
        let probe = |binary, version: Option<&str>, writable| AgentProbe {
            binary,
            version: version.map(str::to_owned),
            sessions_dir: PathBuf::from("/run/user/1000/hyprpilot/sessions"),
            sessions_writable: writable,
            instances_dir: PathBuf::from("/run/user/1000/hypr"),
            instances_listable: true,
            host_socket: Some(true),
        };
        let lines = |probe: &AgentProbe| {
            let mut report = Report::default();
            agent_checks(probe, &mut report);
            report.finish()
        };

        let report = lines(&probe(
            true,
            Some("Hyprland 0.56.0 built from branch main at commit abc"),
            true,
        ))?;
        assert!(report.contains("ok    Hyprland found on PATH"), "{report}");
        assert!(
            report.contains("ok    Hyprland 0.56.0 is the version"),
            "{report}"
        );
        assert!(
            report.contains("ok    /run/user/1000/hyprpilot/sessions is writable"),
            "{report}"
        );

        // Another version is a warning, not a failure.
        let report = lines(&probe(
            true,
            Some("Hyprland 0.49.0 built from branch"),
            true,
        ))?;
        assert!(report.contains("warn  Hyprland 0.49.0"), "{report}");
        assert!(report.contains(TESTED_HYPRLAND), "{report}");
        let report = lines(&probe(true, None, true))?;
        assert!(report.contains("warn  no version number"), "{report}");

        // A missing binary or an unwritable session directory is a failure.
        let error = lines(&probe(false, Some("Hyprland 0.56.0"), false))
            .err()
            .ok_or("a broken environment passed doctor")?;
        assert!(
            matches!(error, Error::DoctorFailed { failures: 2, .. }),
            "{error:?}"
        );
        let report = error.to_string();
        assert!(
            report.contains("FAIL  Hyprland not found on PATH"),
            "{report}"
        );
        assert!(report.contains("session start --isolated"), "{report}");
        assert!(report.contains("is not writable"), "{report}");

        // `doctor` lists the sockets: a start discovers its compositor by
        // diffing $XDG_RUNTIME_DIR/hypr, and every `hyprctl -i` reaches it
        // through the socket inside — neither is possible if that directory
        // cannot be read.
        let report = lines(&probe(true, Some("Hyprland 0.56.0"), true))?;
        assert!(
            report.contains("ok    /run/user/1000/hypr is listable"),
            "{report}"
        );
        assert!(
            report.contains("ok    this session's own instance socket"),
            "{report}"
        );

        let error = lines(&AgentProbe {
            instances_listable: false,
            host_socket: Some(false),
            ..probe(true, Some("Hyprland 0.56.0"), true)
        })
        .err()
        .ok_or("an unreadable instance directory passed doctor")?;
        assert!(
            matches!(error, Error::DoctorFailed { failures: 2, .. }),
            "{error:?}"
        );
        let report = error.to_string();
        assert!(
            report.contains("FAIL  /run/user/1000/hypr cannot be listed"),
            "{report}"
        );
        assert!(report.contains("instance socket is missing"), "{report}");

        // Without a signature there is no socket path to check, and the missing
        // signature is reported on its own.
        let report = lines(&AgentProbe {
            host_socket: None,
            ..probe(true, Some("Hyprland 0.56.0"), true)
        })?;
        assert!(!report.contains("instance socket"), "{report}");
        Ok(())
    }

    #[test]
    fn a_version_number_is_read_out_of_the_hyprctl_line() {
        assert_eq!(
            version_number("Hyprland 0.56.0 built from branch main at commit deadbeef"),
            Some("0.56.0")
        );
        assert_eq!(version_number("Hyprland, v0.45.2"), Some("0.45.2"));
        assert_eq!(version_number("Hyprland built from source"), None);
        assert_eq!(version_number(""), None);
    }
}
