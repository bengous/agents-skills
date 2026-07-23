use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::error::Error;
use crate::{capture, hypr, keys, pointer, session};

#[derive(Parser)]
#[command(
    name = "hyprpilot",
    version,
    about = "Drive and inspect a native GUI app on a headless Hyprland output, \
             without touching the user's desktop"
)]
struct Cli {
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
        /// Output directory (default: `$XDG_RUNTIME_DIR/hyprpilot/shots`)
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
        /// Command to launch if no window matches yet
        #[arg(long, value_name = "CMD")]
        app: Option<String>,
        /// Exact window title to attach to
        #[arg(long, value_name = "TITLE")]
        match_title: Option<String>,
        /// Exact window class to attach to
        #[arg(long, value_name = "CLASS")]
        match_class: Option<String>,
        /// Headless output resolution
        #[arg(long, default_value = "1600x1000")]
        size: String,
    },
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
    match Cli::parse().command {
        Command::Session(SessionCommand::Start {
            app,
            match_title,
            match_class,
            size,
        }) => session::start(
            app.as_deref(),
            match_title.as_deref(),
            match_class.as_deref(),
            &size,
        ),
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
            session::target(
                address.as_deref(),
                match_title.as_deref(),
                match_class.as_deref(),
                pid,
                untracked,
                wait,
                on_teardown.map(Into::into),
            )
        }
        Command::Key {
            keys,
            delay_ms,
            focus,
        } => crate::keys::send_keys(&keys, delay_ms, focus),
        Command::Type {
            text,
            delay_ms,
            focus,
        } => keys::type_text(&text, delay_ms, focus),
        Command::Click {
            x,
            y,
            button,
            double,
            absolute,
            focus,
        } => pointer::click(x, y, button.into(), double, absolute, focus),
        Command::Scroll {
            x,
            y,
            dy,
            dx,
            absolute,
            focus,
        } => pointer::scroll(x, y, dx, dy, absolute, focus),
        Command::Shot { name, full, out } => capture::shot(name.as_deref(), full, out.as_deref()),
        Command::Wait {
            changed_from,
            timeout,
            ..
        } => {
            let mode =
                changed_from.map_or(capture::WaitMode::Stable, capture::WaitMode::ChangedFrom);
            let timeout = capture::parse_timeout(&timeout)?;
            capture::wait(&mode, timeout)
        }
        Command::Status => status(),
        Command::Windows => windows(),
        Command::Doctor => doctor(),
        Command::Teardown { kill, close } => session::teardown(kill, close),
    }
}

enum WindowSession {
    Absent,
    Valid(session::Session),
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
                WindowSession::Valid(session) => (
                    Some(
                        session
                            .windows
                            .iter()
                            .any(|window| window.address == client.address),
                    ),
                    Some(session.active_address == client.address),
                ),
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

fn windows() -> Result<String, Error> {
    let clients = hypr::clients()?;
    let focused = hypr::active_window()?;
    let session = match session::load() {
        Ok(session) => WindowSession::Valid(session),
        Err(Error::NoSession) => WindowSession::Absent,
        Err(
            error @ (Error::Json { .. }
            | Error::CorruptSession { .. }
            | Error::UnsupportedSessionVersion(_)),
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
    serialize_windows(&clients, focused.as_ref(), &session)
}

fn status() -> Result<String, Error> {
    let state = session::load()?;
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

    let value = serde_json::json!({
        "schema_version": state.schema_version,
        "windows": &state.windows,
        "active_address": &state.active_address,
        "parked_windows": parked_windows,
        "configured_size": state.size,
        "effective_size": effective_size,
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
        "attached": state.spawned_pid.is_none(),
        "spawned_pid": state.spawned_pid,
    });
    serde_json::to_string_pretty(&value).map_err(|source| Error::Json {
        context: "serializing status".to_owned(),
        source,
    })
}

fn on_path(binary: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|path| env::split_paths(&path).any(|dir| dir.join(binary).is_file()))
}

fn doctor() -> Result<String, Error> {
    let mut lines: Vec<String> = Vec::new();
    let mut failures = 0usize;

    let mut check = |ok: bool, success: String, failure: String| {
        if ok {
            lines.push(format!("ok    {success}"));
        } else {
            failures += 1;
            lines.push(format!("FAIL  {failure}"));
        }
    };

    match hypr::version() {
        Ok(version) => {
            let first_line = version.lines().next().unwrap_or("unknown").to_owned();
            check(true, first_line, String::new());
        }
        Err(error) => check(false, String::new(), format!("hyprctl: {error}")),
    }

    check(
        env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
        "HYPRLAND_INSTANCE_SIGNATURE is set".to_owned(),
        "HYPRLAND_INSTANCE_SIGNATURE is not set — not inside a Hyprland session?".to_owned(),
    );

    check(
        on_path("grim"),
        "grim found on PATH".to_owned(),
        "grim not found on PATH — install grim for captures".to_owned(),
    );

    match session::runtime_dir() {
        Ok(dir) => {
            let probe = dir.join(".doctor-probe");
            let writable = fs::create_dir_all(&dir).is_ok()
                && fs::write(&probe, b"probe").is_ok()
                && fs::remove_file(&probe).is_ok();
            check(
                writable,
                format!("{} is writable", dir.display()),
                format!("{} is not writable", dir.display()),
            );
        }
        Err(error) => check(false, String::new(), error.to_string()),
    }

    match pointer::probe_virtual_pointer() {
        Ok(present) => check(
            present,
            "zwlr_virtual_pointer_manager_v1 is available".to_owned(),
            "compositor does not expose zwlr_virtual_pointer_manager_v1 — `click` will not work"
                .to_owned(),
        ),
        Err(error) => check(false, String::new(), error.to_string()),
    }

    match hypr::devices() {
        Ok(devices) => {
            let (layout, keymap) = devices.keyboards.iter().find(|k| k.main).map_or_else(
                || ("unknown".to_owned(), "unknown".to_owned()),
                |k| (k.layout.clone(), k.active_keymap.clone()),
            );
            lines.push(format!(
                "info  main keyboard layout `{layout}` (active keymap `{keymap}`) — `type` \
                 maps characters through US shift pairs; accented characters need a keymap \
                 exposing them (e.g. fr)"
            ));
        }
        Err(error) => {
            failures += 1;
            lines.push(format!("FAIL  hyprctl devices: {error}"));
        }
    }

    let report = lines.join("\n");
    if failures == 0 {
        Ok(report)
    } else {
        Err(Error::DoctorFailed { report, failures })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error as StdError;

    use clap::Parser;

    use super::{Cli, Command, WindowSession, serialize_windows};
    use crate::hypr::Client;
    use crate::session::{Disposition, Session, TrackedWindow};

    const CLIENTS_JSON: &str = include_str!("../fixtures/clients.json");

    fn clients() -> Result<Vec<Client>, serde_json::Error> {
        serde_json::from_str(CLIENTS_JSON)
    }

    fn tracked(client: &Client) -> TrackedWindow {
        TrackedWindow {
            address: client.address.clone(),
            title_at_adoption: client.title.clone(),
            origin_workspace: client.workspace.name.clone(),
            origin_at: client.at,
            origin_size: client.size,
            origin_floating: client.floating,
            teardown: Disposition::Restore,
        }
    }

    fn valid_session(clients: &[Client]) -> Session {
        Session {
            schema_version: 2,
            output: "hyprpilot".to_owned(),
            output_created: true,
            active_workspace: "hyprpilot".to_owned(),
            parking_workspace: "special:hyprpilot-parked".to_owned(),
            size: [1600, 1000],
            spawned_pid: None,
            initial_user_focus: None,
            primary_address: clients[0].address.clone(),
            active_address: clients[1].address.clone(),
            windows: vec![tracked(&clients[0]), tracked(&clients[1])],
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
                command: Command::Target(_)
            })
        ));
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
                command: Command::Target(_)
            })
        ));
    }

    #[test]
    fn action_commands_parse_focus_flag() {
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "key", "--focus", "Return"]),
            Ok(Cli {
                command: Command::Key { focus: true, .. }
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "type", "--focus", "text"]),
            Ok(Cli {
                command: Command::Type { focus: true, .. }
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "click", "--focus", "20", "20"]),
            Ok(Cli {
                command: Command::Click { focus: true, .. }
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["hyprpilot", "scroll", "--focus", "20", "20", "--dy", "1",]),
            Ok(Cli {
                command: Command::Scroll { focus: true, .. }
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
        let session = WindowSession::Valid(valid_session(&clients));
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
}
