//! Session state: tracked windows driven on a dedicated headless output.
//! State lives in `$XDG_RUNTIME_DIR/hyprpilot/session.json`; creating it with
//! `create_new` is the single-session lock, and it is written **before** any
//! compositor side effect so a failed start stays recoverable via `teardown`.

use std::env;
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::hypr;

pub const OUTPUT_NAME: &str = "hyprpilot";
pub const WORKSPACE_NAME: &str = "hyprpilot";
const PARKING_WORKSPACE_NAME: &str = "special:hyprpilot-parked";
const SCHEMA_VERSION: u32 = 2;
const WINDOW_APPEAR_TIMEOUT: Duration = Duration::from_secs(15);
const WINDOW_CLOSE_TIMEOUT: Duration = Duration::from_secs(3);
const WINDOW_PLACE_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(200);
const VERIFIED_PLACEMENT_READS: u8 = 2;

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub schema_version: u32,
    pub output: String,
    /// False when an output with our name already existed and was reused —
    /// teardown then leaves it in place.
    pub output_created: bool,
    pub active_workspace: String,
    pub parking_workspace: String,
    pub size: [u32; 2],
    /// None when attached to a pre-existing window.
    pub spawned_pid: Option<u32>,
    /// Address of the user's focused window when the session started, so
    /// `status` can assert the focus was left untouched.
    pub initial_user_focus: Option<String>,
    pub primary_address: String,
    pub active_address: String,
    pub windows: Vec<TrackedWindow>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackedWindow {
    pub address: String,
    pub title_at_adoption: String,
    pub origin_workspace: String,
    pub origin_at: [i32; 2],
    pub origin_size: [i32; 2],
    pub origin_floating: bool,
    pub teardown: Disposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    Restore,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Keep,
    MoveTo(i32, i32),
    Oversized(i32, i32),
}

pub fn place(window: Rect, output: Rect) -> Placement {
    if window.w > output.w || window.h > output.h {
        return Placement::Oversized(output.x, output.y);
    }
    if contains(output, window) {
        return Placement::Keep;
    }
    Placement::MoveTo(
        window
            .x
            .clamp(output.x, output.x.saturating_add(output.w - window.w)),
        window
            .y
            .clamp(output.y, output.y.saturating_add(output.h - window.h)),
    )
}

#[derive(Deserialize)]
struct SessionVersion {
    schema_version: u32,
}

#[derive(Debug, Deserialize)]
struct LegacySession {
    window_address: String,
    output: String,
    output_created: bool,
    origin_workspace: Option<String>,
    spawned_pid: Option<u32>,
}

enum TeardownSession {
    V2(Session),
    Legacy(LegacySession),
}

impl LegacySession {
    fn attached(&self) -> bool {
        self.spawned_pid.is_none()
    }
}

pub struct CurrentSession {
    pub output: String,
    pub workspace: String,
}

pub fn runtime_dir() -> Result<PathBuf, Error> {
    let base = env::var_os("XDG_RUNTIME_DIR").ok_or(Error::Env("XDG_RUNTIME_DIR"))?;
    Ok(PathBuf::from(base).join("hyprpilot"))
}

pub fn session_path() -> Result<PathBuf, Error> {
    Ok(runtime_dir()?.join("session.json"))
}

fn read_from(path: &Path) -> Result<String, Error> {
    match fs::read_to_string(path) {
        Ok(raw) => Ok(raw),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Err(Error::NoSession),
        Err(source) => Err(Error::Io {
            context: format!("reading session file {}", path.display()),
            source,
        }),
    }
}

fn parse_json<'a, T: Deserialize<'a>>(raw: &'a str, path: &Path) -> Result<T, Error> {
    serde_json::from_str(raw).map_err(|source| Error::Json {
        context: format!("parsing session file {}", path.display()),
        source,
    })
}

fn parse_json_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
    path: &Path,
) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|source| Error::Json {
        context: format!("parsing session file {}", path.display()),
        source,
    })
}

fn load_from(path: &Path) -> Result<Session, Error> {
    let raw = read_from(path)?;
    let version: SessionVersion = parse_json(&raw, path)?;
    if version.schema_version != SCHEMA_VERSION {
        return Err(Error::UnsupportedSessionVersion(version.schema_version));
    }
    parse_json(&raw, path)
}

// Legacy parsing stays separate so no command except teardown can accept the
// old, unversioned format.
fn load_for_teardown_from(path: &Path) -> Result<TeardownSession, Error> {
    let raw = read_from(path)?;
    let corrupt = |error: Error| Error::CorruptSession {
        path: path.to_path_buf(),
        message: error.to_string(),
    };
    let value: serde_json::Value = parse_json(&raw, path).map_err(&corrupt)?;
    if value.get("schema_version").is_some() {
        let version: SessionVersion = parse_json_value(value.clone(), path).map_err(&corrupt)?;
        if version.schema_version != SCHEMA_VERSION {
            return Err(Error::UnsupportedSessionVersion(version.schema_version));
        }
        let session: Session = parse_json_value(value, path).map_err(&corrupt)?;
        if session
            .windows
            .iter()
            .filter(|window| window.address == session.primary_address)
            .count()
            != 1
        {
            return Err(Error::CorruptSession {
                path: path.to_path_buf(),
                message: "primary_address must identify exactly one tracked window".to_owned(),
            });
        }
        Ok(TeardownSession::V2(session))
    } else {
        parse_json_value(value, path)
            .map(TeardownSession::Legacy)
            .map_err(corrupt)
    }
}

fn serialize(session: &Session) -> Result<String, Error> {
    serde_json::to_string_pretty(session).map_err(|source| Error::Json {
        context: "serializing session state".to_owned(),
        source,
    })
}

/// Atomically claims the session lock: fails with `SessionExists` if another
/// session file is already present, without a check-then-write race.
fn save_new_to(path: &Path, session: &Session) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Io {
            context: format!("creating {}", parent.display()),
            source,
        })?;
    }
    let raw = serialize(session)?;
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => file,
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(Error::SessionExists(path.to_path_buf()));
        }
        Err(source) => {
            return Err(Error::Io {
                context: format!("creating session file {}", path.display()),
                source,
            });
        }
    };
    file.write_all(raw.as_bytes()).map_err(|source| Error::Io {
        context: format!("writing session file {}", path.display()),
        source,
    })?;
    file.sync_all().map_err(|source| Error::Io {
        context: format!("syncing session file {}", path.display()),
        source,
    })
}

#[cfg_attr(not(test), expect(dead_code, reason = "required session mutation API"))]
pub fn save_over(path: &Path, session: &Session) -> Result<(), Error> {
    let raw = serialize(session)?;
    let file_name = path.file_name().ok_or_else(|| Error::Invalid {
        what: "session path",
        value: path.display().to_string(),
        hint: "expected a file name".to_owned(),
    })?;
    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(format!(".{}.tmp", std::process::id()));
    let tmp_path = path.with_file_name(tmp_name);

    let write_result = (|| {
        let mut file = fs::File::create(&tmp_path).map_err(|source| Error::Io {
            context: format!("creating temporary session file {}", tmp_path.display()),
            source,
        })?;
        file.write_all(raw.as_bytes()).map_err(|source| Error::Io {
            context: format!("writing temporary session file {}", tmp_path.display()),
            source,
        })?;
        file.sync_all().map_err(|source| Error::Io {
            context: format!("syncing temporary session file {}", tmp_path.display()),
            source,
        })?;
        fs::rename(&tmp_path, path).map_err(|source| Error::Io {
            context: format!(
                "replacing session file {} with {}",
                path.display(),
                tmp_path.display()
            ),
            source,
        })
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(tmp_path);
    }
    write_result
}

pub fn load() -> Result<Session, Error> {
    load_from(&session_path()?)
}

/// The session's active window as Hyprland currently sees it.
pub fn current_window() -> Result<(CurrentSession, hypr::Client), Error> {
    let session = load()?;
    let clients = hypr::clients()?;
    let window = clients
        .into_iter()
        .find(|c| c.address == session.active_address)
        .ok_or_else(|| Error::WindowGone(session.active_address.clone()))?;
    Ok((
        CurrentSession {
            output: session.output,
            workspace: session.active_workspace,
        },
        window,
    ))
}

pub fn find_output(name: &str) -> Result<Option<hypr::Monitor>, Error> {
    Ok(hypr::monitors()?.into_iter().find(|m| m.name == name))
}

pub fn parse_size(raw: &str) -> Result<(u32, u32), Error> {
    let invalid = || Error::Invalid {
        what: "size",
        value: raw.to_owned(),
        hint: "expected WIDTHxHEIGHT, e.g. 1600x1000".to_owned(),
    };
    let (w, h) = raw.split_once('x').ok_or_else(invalid)?;
    let width: u32 = w.trim().parse().map_err(|_| invalid())?;
    let height: u32 = h.trim().parse().map_err(|_| invalid())?;
    if width == 0 || height == 0 {
        return Err(invalid());
    }
    Ok((width, height))
}

/// Selector accepted by workspace dispatchers: numeric names and special
/// workspaces pass through; ordinary named workspaces need `name:`.
pub fn workspace_selector(workspace: &hypr::WorkspaceRef) -> String {
    if workspace.name.parse::<i64>().is_ok() || workspace.name.starts_with("special:") {
        workspace.name.clone()
    } else {
        format!("name:{}", workspace.name)
    }
}

fn matches(client: &hypr::Client, title: Option<&str>, class: Option<&str>) -> bool {
    title.is_none_or(|t| client.title == t) && class.is_none_or(|c| client.class == c)
}

fn criteria_label(title: Option<&str>, class: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(t) = title {
        parts.push(format!("title `{t}`"));
    }
    if let Some(c) = class {
        parts.push(format!("class `{c}`"));
    }
    parts.join(" and ")
}

fn find_window(title: Option<&str>, class: Option<&str>) -> Result<Option<hypr::Client>, Error> {
    Ok(hypr::clients()?
        .into_iter()
        .find(|c| matches(c, title, class)))
}

fn spawn_app(command: &str) -> Result<u32, Error> {
    use std::os::unix::process::CommandExt;
    let child = Command::new("sh")
        .args(["-c", command])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|source| Error::Io {
            context: format!("spawning `{command}`"),
            source,
        })?;
    Ok(child.id())
}

fn wait_for_window(
    title: Option<&str>,
    class: Option<&str>,
) -> Result<Option<hypr::Client>, Error> {
    let deadline = Instant::now() + WINDOW_APPEAR_TIMEOUT;
    loop {
        if let Some(window) = find_window(title, class)? {
            return Ok(Some(window));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn client_rect(client: &hypr::Client) -> Rect {
    Rect {
        x: client.at[0],
        y: client.at[1],
        w: client.size[0],
        h: client.size[1],
    }
}

fn exact_layout_integer(value: f64, field: &str, output: &str) -> Result<i32, Error> {
    if !value.is_finite()
        || value < f64::from(i32::MIN)
        || value > f64::from(i32::MAX)
        || value.fract().abs() > f64::EPSILON
    {
        return Err(Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("output {output} reports non-integer {field} {value}"),
        });
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite integral value was range-checked above"
    )]
    let value = value as i32;
    Ok(value)
}

fn output_rect(output: &hypr::Monitor) -> Result<Rect, Error> {
    let (width, height) = output.logical_size();
    Ok(Rect {
        x: exact_layout_integer(output.x, "x", &output.name)?,
        y: exact_layout_integer(output.y, "y", &output.name)?,
        w: exact_layout_integer(width, "logical width", &output.name)?,
        h: exact_layout_integer(height, "logical height", &output.name)?,
    })
}

fn right(rect: Rect) -> i64 {
    i64::from(rect.x) + i64::from(rect.w)
}

fn bottom(rect: Rect) -> i64 {
    i64::from(rect.y) + i64::from(rect.h)
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && right(inner) <= right(outer)
        && bottom(inner) <= bottom(outer)
}

fn oversized_overlap_is_verified(window: Rect, output: Rect) -> bool {
    window.x == output.x
        && window.y == output.y
        && if window.w > output.w {
            right(window) >= right(output)
        } else {
            right(window) <= right(output)
        }
        && if window.h > output.h {
            bottom(window) >= bottom(output)
        } else {
            bottom(window) <= bottom(output)
        }
}

fn read_park_state(
    address: &str,
    output_name: &str,
) -> Result<(hypr::Client, hypr::Monitor), Error> {
    let window = hypr::clients()?
        .into_iter()
        .find(|client| client.address == address)
        .ok_or_else(|| Error::WindowGone(address.to_owned()))?;
    let output = hypr::monitors()?
        .into_iter()
        .find(|monitor| monitor.name == output_name)
        .ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("session output {output_name} is missing"),
        })?;
    Ok((window, output))
}

fn wait_for_session_workspace(
    address: &str,
    output_name: &str,
    workspace_name: &str,
) -> Result<(hypr::Client, hypr::Monitor), Error> {
    let deadline = Instant::now() + WINDOW_PLACE_TIMEOUT;
    loop {
        let state = read_park_state(address, output_name)?;
        if state.0.workspace.name == workspace_name {
            return Ok(state);
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout {
                what: format!(
                    "window {address} to enter workspace {workspace_name} (last observed: {})",
                    state.0.workspace.name
                ),
                after_ms: WINDOW_PLACE_TIMEOUT.as_millis(),
            });
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn placement_is_verified(
    window: &hypr::Client,
    output: &hypr::Monitor,
    workspace_name: &str,
    placement: Placement,
) -> Result<bool, Error> {
    if window.workspace.name != workspace_name
        || output.active_workspace.name != workspace_name
        || !output.special_workspace.is_empty()
    {
        return Ok(false);
    }
    if !window.floating {
        return Ok(true);
    }
    let window = client_rect(window);
    let output = output_rect(output)?;
    Ok(match placement {
        Placement::Keep | Placement::MoveTo(_, _) => contains(output, window),
        Placement::Oversized(_, _) => oversized_overlap_is_verified(window, output),
    })
}

fn wait_for_verified_placement(
    address: &str,
    output_name: &str,
    workspace_name: &str,
    placement: Placement,
) -> Result<(), Error> {
    let deadline = Instant::now() + WINDOW_PLACE_TIMEOUT;
    let mut verified_reads = 0;
    loop {
        let (window, output) = read_park_state(address, output_name)?;
        if placement_is_verified(&window, &output, workspace_name, placement)? {
            verified_reads += 1;
            if verified_reads == VERIFIED_PLACEMENT_READS {
                return Ok(());
            }
        } else {
            verified_reads = 0;
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout {
                what: format!(
                    "verified placement of window {address} on output {output_name} \
                     (last observed: workspace {}, floating {}, at {:?}, size {:?}; \
                     output at ({}, {}), logical size {:?})",
                    window.workspace.name,
                    window.floating,
                    window.at,
                    window.size,
                    output.x,
                    output.y,
                    output.logical_size(),
                ),
                after_ms: WINDOW_PLACE_TIMEOUT.as_millis(),
            });
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn evacuate_stray_workspace(output_name: &str, workspace_name: &str) -> Result<(), Error> {
    let monitors = hypr::monitors()?;
    let ours = monitors
        .iter()
        .find(|monitor| monitor.name == output_name)
        .ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("output {output_name} missing right after creation"),
        })?;
    if ours.active_workspace.name == workspace_name {
        return Ok(());
    }
    let refuge = monitors
        .iter()
        .find(|monitor| monitor.name != output_name)
        .ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: "no other monitor to evacuate the stray workspace to".to_owned(),
        })?;
    hypr::dispatch(&[
        "moveworkspacetomonitor",
        &workspace_selector(&ours.active_workspace),
        &refuge.name,
    ])
}

fn park_window(
    address: &str,
    output_name: &str,
    workspace_name: &str,
) -> Result<Option<Placement>, Error> {
    hypr::dispatch(&[
        "movetoworkspacesilent",
        &format!("name:{workspace_name},address:{address}"),
    ])?;
    hypr::dispatch(&[
        "moveworkspacetomonitor",
        &format!("name:{workspace_name}"),
        output_name,
    ])?;
    evacuate_stray_workspace(output_name, workspace_name)?;

    let (window, output) = wait_for_session_workspace(address, output_name, workspace_name)?;
    let placement = place(client_rect(&window), output_rect(&output)?);
    if window.floating {
        match placement {
            Placement::Keep => {}
            Placement::MoveTo(x, y) | Placement::Oversized(x, y) => {
                hypr::dispatch(&[
                    "movewindowpixel",
                    &format!("exact {x} {y},address:{address}"),
                ])?;
            }
        }
    }
    wait_for_verified_placement(address, output_name, workspace_name, placement)?;
    Ok(window.floating.then_some(placement))
}

pub fn start(
    app: Option<&str>,
    match_title: Option<&str>,
    match_class: Option<&str>,
    size: &str,
) -> Result<String, Error> {
    if match_title.is_none() && match_class.is_none() {
        return Err(Error::Invalid {
            what: "match criteria",
            value: "(none)".to_owned(),
            hint: "pass --match-title and/or --match-class".to_owned(),
        });
    }
    let (width, height) = parse_size(size)?;
    let path = session_path()?;
    // Courtesy fast path so we do not spawn an app just to kill it; the
    // atomic `save_new_to` below remains the authoritative lock.
    if path.exists() {
        return Err(Error::SessionExists(path));
    }

    // Captured before any side effect, so status/teardown can reason about
    // what the user had.
    let initial_user_focus = hypr::active_window()?.map(|w| w.address);

    let criteria = criteria_label(match_title, match_class);
    let mut spawned_pid = None;
    let window = if let Some(window) = find_window(match_title, match_class)? {
        window
    } else {
        let Some(command) = app else {
            return Err(Error::WindowNotFound(format!(
                "{criteria} — pass --app to launch it"
            )));
        };
        let pid = spawn_app(command)?;
        spawned_pid = Some(pid);
        let Some(window) = wait_for_window(match_title, match_class)? else {
            // Do not leak the app we just launched.
            let _ = kill_process_group(pid);
            return Err(Error::WindowNotFound(format!(
                "{criteria} after launching `{command}` ({}s timeout) — process killed",
                WINDOW_APPEAR_TIMEOUT.as_secs()
            )));
        };
        window
    };

    let output_created = find_output(OUTPUT_NAME)?.is_none();
    let teardown = spawned_pid.map_or(Disposition::Restore, |_| Disposition::Close);
    let session = Session {
        schema_version: SCHEMA_VERSION,
        output: OUTPUT_NAME.to_owned(),
        output_created,
        active_workspace: WORKSPACE_NAME.to_owned(),
        parking_workspace: PARKING_WORKSPACE_NAME.to_owned(),
        size: [width, height],
        spawned_pid,
        initial_user_focus,
        primary_address: window.address.clone(),
        active_address: window.address.clone(),
        windows: vec![TrackedWindow {
            address: window.address.clone(),
            title_at_adoption: window.title.clone(),
            origin_workspace: window.workspace.name.clone(),
            origin_at: window.at,
            origin_size: window.size,
            origin_floating: window.floating,
            teardown,
        }],
    };
    // Lock + persist before touching the compositor: if anything below
    // fails, `hyprpilot teardown` can still clean up from this state.
    if let Err(error) = save_new_to(&path, &session) {
        if let (Error::SessionExists(_), Some(pid)) = (&error, spawned_pid) {
            let _ = kill_process_group(pid);
        }
        return Err(error);
    }

    if output_created {
        hypr::output_create_headless(OUTPUT_NAME)?;
    }
    hypr::keyword_monitor(OUTPUT_NAME, width, height)?;

    if matches!(
        park_window(&window.address, OUTPUT_NAME, WORKSPACE_NAME)?,
        Some(Placement::Oversized(_, _))
    ) {
        let _ = writeln!(
            std::io::stderr(),
            "hyprpilot: warning: window {} is larger than output {OUTPUT_NAME}; \
             use `hyprpilot session resize` when available",
            window.address
        );
    }

    Ok(format!(
        "session ready — window {} (`{}`) parked on output {OUTPUT_NAME} ({width}x{height}), workspace {WORKSPACE_NAME}",
        window.address, window.title
    ))
}

fn kill_process_group(pid: u32) -> Result<(), Error> {
    let output = Command::new("kill")
        .args(["--", &format!("-{pid}")])
        .output()
        .map_err(|source| Error::Io {
            context: format!("running `kill -- -{pid}`"),
            source,
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Tool {
            command: format!("kill -- -{pid}"),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

fn window_exists(address: &str) -> Result<bool, Error> {
    Ok(hypr::clients()?.iter().any(|c| c.address == address))
}

/// Waits for the window to disappear; on timeout returns an error so the
/// caller aborts teardown instead of dropping a live window onto the user's
/// desktop by removing its output underneath it.
fn wait_window_gone(address: &str, hint: &str) -> Result<(), Error> {
    let deadline = Instant::now() + WINDOW_CLOSE_TIMEOUT;
    while window_exists(address)? {
        if Instant::now() >= deadline {
            return Err(Error::Timeout {
                what: format!("window {address} to close ({hint})"),
                after_ms: WINDOW_CLOSE_TIMEOUT.as_millis(),
            });
        }
        thread::sleep(POLL_INTERVAL);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowAction {
    Restore,
    Close,
    Kill(u32),
}

fn validate_teardown_flags(spawned_pid: Option<u32>, kill: bool, close: bool) -> Result<(), Error> {
    if kill && close {
        return Err(Error::Invalid {
            what: "teardown flags",
            value: "--kill --close".to_owned(),
            hint: "--kill and --close are mutually exclusive".to_owned(),
        });
    }
    if kill && spawned_pid.is_none() {
        return Err(Error::Invalid {
            what: "teardown flag",
            value: "--kill".to_owned(),
            hint: "--kill requires a spawned session with a spawned_pid".to_owned(),
        });
    }
    if close && spawned_pid.is_some() {
        return Err(Error::Invalid {
            what: "teardown flag",
            value: "--close".to_owned(),
            hint: "--close only applies to an attached primary; spawned sessions close by default"
                .to_owned(),
        });
    }
    Ok(())
}

fn teardown_plan(
    session: &Session,
    kill: bool,
    close: bool,
) -> Result<Vec<(&TrackedWindow, WindowAction)>, Error> {
    validate_teardown_flags(session.spawned_pid, kill, close)?;
    let kill_pid = kill.then_some(session.spawned_pid).flatten();
    Ok(session
        .windows
        .iter()
        .rev()
        .map(|window| {
            let action = if window.address == session.primary_address {
                let default = if close {
                    WindowAction::Close
                } else {
                    match window.teardown {
                        Disposition::Restore => WindowAction::Restore,
                        Disposition::Close => WindowAction::Close,
                    }
                };
                kill_pid.map_or(default, WindowAction::Kill)
            } else {
                match window.teardown {
                    Disposition::Restore => WindowAction::Restore,
                    Disposition::Close => WindowAction::Close,
                }
            };
            (window, action)
        })
        .collect())
}

fn close_window(address: &str, hint: &str) -> Result<(), Error> {
    hypr::dispatch(&["closewindow", &format!("address:{address}")])?;
    wait_window_gone(address, hint)
}

fn restore_window(window: &TrackedWindow) -> Result<(), Error> {
    let selector = workspace_selector(&hypr::WorkspaceRef {
        name: window.origin_workspace.clone(),
    });
    hypr::dispatch(&[
        "movetoworkspacesilent",
        &format!("{selector},address:{}", window.address),
    ])?;

    if window.origin_floating {
        // GlobalWindowController.cpp:35-68 recalculates relative position on
        // cross-monitor moves, so geometry must follow the workspace change.
        hypr::dispatch(&[
            "movewindowpixel",
            &format!(
                "exact {} {},address:{}",
                window.origin_at[0], window.origin_at[1], window.address
            ),
        ])?;
        hypr::dispatch(&[
            "resizewindowpixel",
            &format!(
                "exact {} {},address:{}",
                window.origin_size[0], window.origin_size[1], window.address
            ),
        ])?;
    }
    Ok(())
}

fn teardown_v2(session: &Session, kill: bool, close: bool) -> Result<Vec<String>, Error> {
    let mut notes = Vec::new();
    for (window, action) in teardown_plan(session, kill, close)? {
        if !window_exists(&window.address)? {
            notes.push(format!("window {} already gone", window.address));
            continue;
        }
        match action {
            WindowAction::Restore => {
                restore_window(window)?;
                notes.push(format!(
                    "restored window {} to workspace {}",
                    window.address, window.origin_workspace
                ));
            }
            WindowAction::Close => {
                close_window(
                    &window.address,
                    "app may be prompting — retry with --kill if spawned",
                )?;
                notes.push(format!("closed window {}", window.address));
            }
            WindowAction::Kill(pid) => {
                kill_process_group(pid)?;
                wait_window_gone(&window.address, "after kill")?;
                notes.push(format!("killed spawned process group {pid}"));
            }
        }
    }
    Ok(notes)
}

fn teardown_legacy(session: &LegacySession, kill: bool, close: bool) -> Result<Vec<String>, Error> {
    validate_teardown_flags(session.spawned_pid, kill, close)?;
    if !window_exists(&session.window_address)? {
        return Ok(vec!["window already gone".to_owned()]);
    }

    if let (true, Some(pid)) = (kill, session.spawned_pid) {
        kill_process_group(pid)?;
        wait_window_gone(&session.window_address, "after kill")?;
        return Ok(vec![format!("killed spawned process group {pid}")]);
    }
    if close || !session.attached() {
        close_window(
            &session.window_address,
            "app may be prompting — retry with --kill if spawned",
        )?;
        return Ok(vec![format!("closed window {}", session.window_address)]);
    }

    let origin = session.origin_workspace.as_deref().unwrap_or("1");
    let selector = workspace_selector(&hypr::WorkspaceRef {
        name: origin.to_owned(),
    });
    hypr::dispatch(&[
        "movetoworkspacesilent",
        &format!("{selector},address:{}", session.window_address),
    ])?;
    Ok(vec![format!(
        "moved attached window {} back to workspace {origin}",
        session.window_address
    )])
}

fn ensure_output_empty_for_sweep(
    output: &hypr::Monitor,
    monitors: &[hypr::Monitor],
    clients: &[hypr::Client],
) -> Result<(), Error> {
    if output.id < 0 {
        return Err(Error::SweepRefused {
            output: output.name.clone(),
            reason: format!("output reports unexpected monitor id {}", output.id),
        });
    }
    for client in clients {
        if client.monitor == output.id {
            return Err(Error::SweepRefused {
                output: output.name.clone(),
                reason: format!(
                    "client {} (`{}`) still reports monitor {}",
                    client.address, client.title, client.monitor
                ),
            });
        }
        if client.monitor < 0 || !monitors.iter().any(|monitor| monitor.id == client.monitor) {
            return Err(Error::SweepRefused {
                output: output.name.clone(),
                reason: format!(
                    "client {} (`{}`) reports unexpected monitor {}",
                    client.address, client.title, client.monitor
                ),
            });
        }
    }
    Ok(())
}

fn sweep_orphan_output() -> Result<String, Error> {
    let monitors = hypr::monitors()?;
    let Some(output) = monitors.iter().find(|monitor| monitor.name == OUTPUT_NAME) else {
        return Err(Error::NoSession);
    };
    let clients = hypr::clients()?;
    ensure_output_empty_for_sweep(output, &monitors, &clients)?;
    hypr::output_remove(OUTPUT_NAME)?;
    Ok(format!(
        "no active session, removed empty orphan output {OUTPUT_NAME}"
    ))
}

fn finish_teardown(
    path: &Path,
    output_name: &str,
    output_created: bool,
    mut notes: Vec<String>,
) -> Result<String, Error> {
    if !output_created {
        notes.push(format!("output {output_name} pre-existed — left in place"));
    } else if find_output(output_name)?.is_some() {
        hypr::output_remove(output_name)?;
        notes.push(format!("removed output {output_name}"));
    } else {
        notes.push(format!("output {output_name} already absent"));
    }

    fs::remove_file(path).map_err(|source| Error::Io {
        context: format!("removing session file {}", path.display()),
        source,
    })?;
    notes.push("session state cleared".to_owned());
    Ok(format!("teardown done — {}", notes.join(", ")))
}

pub fn teardown(kill: bool, close: bool) -> Result<String, Error> {
    let path = session_path()?;
    let session = match load_for_teardown_from(&path) {
        Ok(session) => session,
        Err(Error::NoSession) => return sweep_orphan_output(),
        Err(error) => return Err(error),
    };

    match session {
        TeardownSession::V2(session) => {
            let notes = teardown_v2(&session, kill, close)?;
            finish_teardown(&path, &session.output, session.output_created, notes)
        }
        TeardownSession::Legacy(session) => {
            let notes = teardown_legacy(&session, kill, close)?;
            finish_teardown(&path, &session.output, session.output_created, notes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Disposition, Placement, Rect, Session, TeardownSession, TrackedWindow, WindowAction,
        ensure_output_empty_for_sweep, load_for_teardown_from, load_from, parse_size, place,
        save_new_to, save_over, teardown_plan, workspace_selector,
    };
    use crate::error::Error;
    use crate::hypr::{Client, Monitor, WorkspaceRef};
    use std::error::Error as StdError;

    const MONITORS_JSON: &str = include_str!("../fixtures/monitors.json");
    const SWEEP_OCCUPIED_JSON: &str =
        include_str!("../fixtures/sweep-clients-output-occupied.json");
    const SWEEP_EMPTY_JSON: &str = include_str!("../fixtures/sweep-clients-output-empty.json");
    const SWEEP_MINUS_ONE_JSON: &str =
        include_str!("../fixtures/sweep-clients-monitor-minus-one.json");

    fn sample_session() -> Session {
        Session {
            schema_version: 2,
            output: "hyprpilot".to_owned(),
            output_created: true,
            active_workspace: "hyprpilot".to_owned(),
            parking_workspace: "special:hyprpilot-parked".to_owned(),
            size: [1600, 1000],
            spawned_pid: Some(42),
            initial_user_focus: Some("0xdef".to_owned()),
            primary_address: "0xabc".to_owned(),
            active_address: "0xabc".to_owned(),
            windows: vec![TrackedWindow {
                address: "0xabc".to_owned(),
                title_at_adoption: "App".to_owned(),
                origin_workspace: "3".to_owned(),
                origin_at: [120, 80],
                origin_size: [900, 600],
                origin_floating: true,
                teardown: Disposition::Close,
            }],
        }
    }

    #[test]
    fn size_parses_and_rejects() {
        assert_eq!(parse_size("1600x1000").ok(), Some((1600, 1000)));
        assert_eq!(parse_size(" 800 x 600 ").ok(), Some((800, 600)));
        assert!(parse_size("1600").is_err());
        assert!(parse_size("0x100").is_err());
        assert!(parse_size("axb").is_err());
    }

    #[test]
    fn placement_clamps_all_axes_and_handles_oversized_windows() {
        let rect = |x, y, w, h| Rect { x, y, w, h };
        let output = rect(0, 0, 100, 80);
        let cases = [
            ("contained", rect(10, 20, 30, 20), output, Placement::Keep),
            (
                "overflow x",
                rect(90, 20, 30, 20),
                output,
                Placement::MoveTo(70, 20),
            ),
            (
                "overflow y",
                rect(10, -5, 30, 20),
                output,
                Placement::MoveTo(10, 0),
            ),
            (
                "overflow both",
                rect(-10, 70, 30, 20),
                output,
                Placement::MoveTo(0, 60),
            ),
            (
                "oversized x",
                rect(10, 20, 101, 20),
                output,
                Placement::Oversized(0, 0),
            ),
            (
                "oversized y",
                rect(10, 20, 30, 81),
                output,
                Placement::Oversized(0, 0),
            ),
            (
                "oversized both",
                rect(10, 20, 101, 81),
                output,
                Placement::Oversized(0, 0),
            ),
            (
                "negative output and window",
                rect(-130, -40, 40, 30),
                rect(-100, -80, 100, 80),
                Placement::MoveTo(-100, -40),
            ),
        ];

        for (label, window, output, expected) in cases {
            assert_eq!(place(window, output), expected, "{label}");
        }
    }

    #[test]
    fn workspace_selector_prefixes_named_workspaces() {
        let named = WorkspaceRef {
            name: "proto".to_owned(),
        };
        let numeric = WorkspaceRef {
            name: "5".to_owned(),
        };
        let special = WorkspaceRef {
            name: "special:hyprpilot-parked".to_owned(),
        };
        assert_eq!(workspace_selector(&named), "name:proto");
        assert_eq!(workspace_selector(&numeric), "5");
        assert_eq!(workspace_selector(&special), "special:hyprpilot-parked");
    }

    #[test]
    fn teardown_flag_matrix_matches_session_ownership() -> Result<(), Box<dyn StdError>> {
        let spawned = sample_session();
        assert_eq!(
            teardown_plan(&spawned, false, false)
                .ok()
                .and_then(|plan| plan.first().map(|step| step.1)),
            Some(WindowAction::Close)
        );
        assert_eq!(
            teardown_plan(&spawned, true, false)
                .ok()
                .and_then(|plan| plan.first().map(|step| step.1)),
            Some(WindowAction::Kill(42))
        );
        let spawned_close = teardown_plan(&spawned, false, true)
            .err()
            .ok_or("--close accepted a spawned session")?;
        assert!(spawned_close.to_string().contains("attached primary"));

        let mut attached = sample_session();
        attached.spawned_pid = None;
        attached.windows[0].teardown = Disposition::Restore;
        assert_eq!(
            teardown_plan(&attached, false, false)
                .ok()
                .and_then(|plan| plan.first().map(|step| step.1)),
            Some(WindowAction::Restore)
        );
        assert_eq!(
            teardown_plan(&attached, false, true)
                .ok()
                .and_then(|plan| plan.first().map(|step| step.1)),
            Some(WindowAction::Close)
        );
        let attached_kill = teardown_plan(&attached, true, false)
            .err()
            .ok_or("--kill accepted an attached session")?;
        assert!(attached_kill.to_string().contains("spawned_pid"));

        assert!(teardown_plan(&spawned, true, true).is_err());
        Ok(())
    }

    #[test]
    fn teardown_plan_processes_windows_in_reverse_order() -> Result<(), Box<dyn StdError>> {
        let mut session = sample_session();
        session.windows.push(TrackedWindow {
            address: "0xaux".to_owned(),
            title_at_adoption: "Auxiliary".to_owned(),
            origin_workspace: "special:notes".to_owned(),
            origin_at: [300, 200],
            origin_size: [600, 400],
            origin_floating: true,
            teardown: Disposition::Restore,
        });

        let plan = teardown_plan(&session, false, false)?;
        assert_eq!(plan[0].0.address, "0xaux");
        assert_eq!(plan[0].1, WindowAction::Restore);
        assert_eq!(plan[1].0.address, "0xabc");
        assert_eq!(plan[1].1, WindowAction::Close);
        Ok(())
    }

    #[test]
    fn session_round_trips_through_disk() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        save_new_to(&path, &sample_session())?;
        let loaded = load_from(&path)?;
        assert_eq!(loaded.schema_version, 2);
        assert_eq!(loaded.primary_address, "0xabc");
        assert_eq!(loaded.active_address, "0xabc");
        assert_eq!(loaded.size, [1600, 1000]);
        assert_eq!(loaded.spawned_pid, Some(42));
        assert_eq!(loaded.windows[0].origin_workspace, "3");
        assert_eq!(loaded.windows[0].origin_at, [120, 80]);
        assert_eq!(loaded.windows[0].origin_size, [900, 600]);
        assert!(loaded.windows[0].origin_floating);
        assert_eq!(loaded.windows[0].teardown, Disposition::Close);
        assert_eq!(loaded.initial_user_focus.as_deref(), Some("0xdef"));
        assert!(loaded.output_created);
        Ok(())
    }

    #[test]
    fn session_overwrite_replaces_atomically() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        save_new_to(&path, &sample_session())?;
        let mut updated = sample_session();
        updated.active_address = "0xdef".to_owned();

        save_over(&path, &updated)?;

        assert_eq!(load_from(&path)?.active_address, "0xdef");
        assert!(
            !dir.path()
                .join(format!("session.json.{}.tmp", std::process::id()))
                .exists()
        );
        Ok(())
    }

    #[test]
    fn second_session_is_rejected_atomically() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        save_new_to(&path, &sample_session())?;
        let second = save_new_to(&path, &sample_session());
        assert!(matches!(second, Err(Error::SessionExists(_))));
        Ok(())
    }

    #[test]
    fn legacy_session_is_only_accepted_for_teardown() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "window_address": "0xabc",
                "window_title": "App",
                "output": "hyprpilot",
                "output_created": true,
                "workspace": "hyprpilot",
                "origin_workspace": "3",
                "size": [1600, 1000],
                "spawned_pid": null,
                "initial_user_focus": "0xdef"
            }))?,
        )?;

        assert!(matches!(load_from(&path), Err(Error::Json { .. })));
        let TeardownSession::Legacy(legacy) = load_for_teardown_from(&path)? else {
            return Err("legacy file loaded as v2".into());
        };
        assert_eq!(legacy.window_address, "0xabc");
        assert_eq!(legacy.origin_workspace.as_deref(), Some("3"));
        assert!(legacy.attached());
        Ok(())
    }

    #[test]
    fn unknown_session_version_is_rejected_explicitly() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        let mut value = serde_json::to_value(sample_session())?;
        value["schema_version"] = serde_json::json!(3);
        std::fs::write(&path, serde_json::to_vec_pretty(&value)?)?;

        let Err(error) = load_from(&path) else {
            return Err("unknown schema was accepted".into());
        };
        assert!(matches!(&error, Error::UnsupportedSessionVersion(3)));
        assert!(error.to_string().contains("no output was removed"));
        assert!(error.to_string().contains("hyprpilot windows"));
        assert!(matches!(
            load_for_teardown_from(&path),
            Err(Error::UnsupportedSessionVersion(3))
        ));
        Ok(())
    }

    #[test]
    fn missing_session_file_is_no_session() {
        let result = load_from(std::path::Path::new("/nonexistent/session.json"));
        assert!(matches!(result, Err(Error::NoSession)));
    }

    #[test]
    fn corrupt_session_reports_manual_recovery() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        std::fs::write(&path, b"{broken")?;

        let error = load_for_teardown_from(&path)
            .err()
            .ok_or("corrupt session unexpectedly loaded")?;
        assert!(matches!(&error, Error::CorruptSession { .. }));
        let message = error.to_string();
        assert!(message.contains("no output was removed"));
        assert!(message.contains("movetoworkspacesilent"));
        assert!(message.contains("closewindow"));
        assert!(message.contains("output remove hyprpilot"));
        Ok(())
    }

    fn sweep_fixture(clients_json: &str) -> Result<(Vec<Monitor>, Vec<Client>), Box<dyn StdError>> {
        Ok((
            serde_json::from_str(MONITORS_JSON)?,
            serde_json::from_str(clients_json)?,
        ))
    }

    #[test]
    fn sweep_refuses_occupied_output_fixture() -> Result<(), Box<dyn StdError>> {
        let (monitors, clients) = sweep_fixture(SWEEP_OCCUPIED_JSON)?;
        let error = ensure_output_empty_for_sweep(&monitors[1], &monitors, &clients)
            .err()
            .ok_or("occupied output unexpectedly accepted for sweep")?;
        assert!(error.to_string().contains("still reports monitor 1"));
        Ok(())
    }

    #[test]
    fn sweep_accepts_empty_output_fixture() -> Result<(), Box<dyn StdError>> {
        let (monitors, clients) = sweep_fixture(SWEEP_EMPTY_JSON)?;
        ensure_output_empty_for_sweep(&monitors[1], &monitors, &clients)?;
        Ok(())
    }

    #[test]
    fn sweep_refuses_monitor_minus_one_fixture() -> Result<(), Box<dyn StdError>> {
        let (monitors, clients) = sweep_fixture(SWEEP_MINUS_ONE_JSON)?;
        let error = ensure_output_empty_for_sweep(&monitors[1], &monitors, &clients)
            .err()
            .ok_or("monitor -1 unexpectedly accepted for sweep")?;
        assert!(error.to_string().contains("unexpected monitor -1"));
        Ok(())
    }
}
