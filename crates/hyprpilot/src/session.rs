//! Session state: one driven window parked on a dedicated headless output.
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
const WINDOW_APPEAR_TIMEOUT: Duration = Duration::from_secs(15);
const WINDOW_CLOSE_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub window_address: String,
    pub window_title: String,
    pub output: String,
    /// False when an output with our name already existed and was reused —
    /// teardown then leaves it in place.
    pub output_created: bool,
    pub workspace: String,
    /// For attached (not spawned) windows: the workspace to move the window
    /// back to on teardown instead of closing an app we do not own.
    pub origin_workspace: Option<String>,
    pub size: [u32; 2],
    /// None when attached to a pre-existing window.
    pub spawned_pid: Option<u32>,
    /// Address of the user's focused window when the session started, so
    /// `status` can assert the focus was left untouched.
    pub initial_user_focus: Option<String>,
}

impl Session {
    fn attached(&self) -> bool {
        self.spawned_pid.is_none()
    }
}

pub fn runtime_dir() -> Result<PathBuf, Error> {
    let base = env::var_os("XDG_RUNTIME_DIR").ok_or(Error::Env("XDG_RUNTIME_DIR"))?;
    Ok(PathBuf::from(base).join("hyprpilot"))
}

pub fn session_path() -> Result<PathBuf, Error> {
    Ok(runtime_dir()?.join("session.json"))
}

fn load_from(path: &Path) -> Result<Session, Error> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::NoSession);
        }
        Err(source) => {
            return Err(Error::Io {
                context: format!("reading session file {}", path.display()),
                source,
            });
        }
    };
    serde_json::from_str(&raw).map_err(|source| Error::Json {
        context: format!("parsing session file {}", path.display()),
        source,
    })
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
    })
}

pub fn load() -> Result<Session, Error> {
    load_from(&session_path()?)
}

/// The session's window as Hyprland currently sees it.
pub fn current_window() -> Result<(Session, hypr::Client), Error> {
    let session = load()?;
    let clients = hypr::clients()?;
    let window = clients
        .into_iter()
        .find(|c| c.address == session.window_address)
        .ok_or_else(|| Error::WindowGone(session.window_address.clone()))?;
    Ok((session, window))
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

/// Selector accepted by workspace dispatchers: numeric names pass through
/// (they double as ids), anything else needs the `name:` prefix.
pub fn workspace_selector(workspace: &hypr::WorkspaceRef) -> String {
    if workspace.name.parse::<i64>().is_ok() {
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
    let session = Session {
        window_address: window.address.clone(),
        window_title: window.title.clone(),
        output: OUTPUT_NAME.to_owned(),
        output_created,
        workspace: WORKSPACE_NAME.to_owned(),
        origin_workspace: spawned_pid.is_none().then(|| window.workspace.name.clone()),
        size: [width, height],
        spawned_pid,
        initial_user_focus,
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

    hypr::dispatch(&[
        "movetoworkspacesilent",
        &format!("name:{WORKSPACE_NAME},address:{}", window.address),
    ])?;
    hypr::dispatch(&[
        "moveworkspacetomonitor",
        &format!("name:{WORKSPACE_NAME}"),
        OUTPUT_NAME,
    ])?;

    // If the headless output woke up on an empty workspace, evacuate it to a
    // physical monitor — otherwise grim would capture the wallpaper instead
    // of the parked window.
    let monitors = hypr::monitors()?;
    let ours = monitors
        .iter()
        .find(|m| m.name == OUTPUT_NAME)
        .ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("output {OUTPUT_NAME} missing right after creation"),
        })?;
    if ours.active_workspace.name != WORKSPACE_NAME {
        let refuge = monitors
            .iter()
            .find(|m| m.name != OUTPUT_NAME)
            .ok_or_else(|| Error::Tool {
                command: "hyprctl monitors".to_owned(),
                message: "no other monitor to evacuate the stray workspace to".to_owned(),
            })?;
        hypr::dispatch(&[
            "moveworkspacetomonitor",
            &workspace_selector(&ours.active_workspace),
            &refuge.name,
        ])?;
    }

    Ok(format!(
        "session ready — window {} (`{}`) parked on output {OUTPUT_NAME} ({width}x{height}), workspace {WORKSPACE_NAME}",
        window.address, window.title
    ))
}

pub fn status() -> Result<String, Error> {
    let (session, window) = current_window()?;
    let output = find_output(&session.output)?;
    let active = hypr::active_window()?;

    let value = serde_json::json!({
        "window": {
            "address": window.address,
            "title": window.title,
            "class": window.class,
            "at": window.at,
            "size": window.size,
            "workspace": window.workspace.name,
            "pid": window.pid,
        },
        "output": output.map(|m| serde_json::json!({
            "name": m.name,
            "x": m.x,
            "y": m.y,
            "width": m.width,
            "height": m.height,
            "active_workspace": m.active_workspace.name,
        })),
        "user_active_window": active.map(|w| serde_json::json!({
            "address": w.address,
            "title": w.title,
        })),
        "initial_user_focus": session.initial_user_focus,
        "attached": session.attached(),
        "spawned_pid": session.spawned_pid,
    });
    serde_json::to_string_pretty(&value).map_err(|source| Error::Json {
        context: "serializing status".to_owned(),
        source,
    })
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

pub fn teardown(kill: bool, close: bool) -> Result<String, Error> {
    let path = session_path()?;
    // Only an unparseable session file falls back to best-effort cleanup by
    // the well-known output name; I/O errors propagate untouched.
    let session = match load_from(&path) {
        Ok(session) => Some(session),
        Err(Error::Json { .. }) => None,
        // No session but a stray output bearing our name: sweep it anyway
        // (our namespace) instead of leaving it in the user's layout.
        Err(Error::NoSession) => {
            if find_output(OUTPUT_NAME)?.is_some() {
                hypr::output_remove(OUTPUT_NAME)?;
                return Ok(format!(
                    "no active session, but removed stray output {OUTPUT_NAME}"
                ));
            }
            return Err(Error::NoSession);
        }
        Err(error) => return Err(error),
    };

    let mut notes = Vec::new();

    if let Some(session) = &session {
        if window_exists(&session.window_address)? {
            let close_window = close || !session.attached();
            match (kill, session.spawned_pid) {
                (true, Some(pid)) => {
                    kill_process_group(pid)?;
                    notes.push(format!("killed spawned process group {pid}"));
                    wait_window_gone(&session.window_address, "after kill")?;
                }
                _ if close_window => {
                    hypr::dispatch(&[
                        "closewindow",
                        &format!("address:{}", session.window_address),
                    ])?;
                    notes.push(format!("closed window {}", session.window_address));
                    wait_window_gone(
                        &session.window_address,
                        "app may be prompting — retry with --kill if spawned",
                    )?;
                }
                _ => {
                    // Attached window: give it back instead of closing an app
                    // the user had opened themselves.
                    let origin = session.origin_workspace.as_deref().unwrap_or("1");
                    let selector = workspace_selector(&hypr::WorkspaceRef {
                        name: origin.to_owned(),
                    });
                    hypr::dispatch(&[
                        "movetoworkspacesilent",
                        &format!("{selector},address:{}", session.window_address),
                    ])?;
                    notes.push(format!(
                        "moved attached window {} back to workspace {origin}",
                        session.window_address
                    ));
                }
            }
        } else {
            notes.push("window already gone".to_owned());
        }
    } else {
        notes.push("session file was corrupt — cleaning up by output name".to_owned());
    }

    let output_name = session.as_ref().map_or(OUTPUT_NAME, |s| s.output.as_str());
    let output_owned = session.as_ref().is_none_or(|s| s.output_created);
    if !output_owned {
        notes.push(format!("output {output_name} pre-existed — left in place"));
    } else if find_output(output_name)?.is_some() {
        hypr::output_remove(output_name)?;
        notes.push(format!("removed output {output_name}"));
    } else {
        notes.push(format!("output {output_name} already absent"));
    }

    fs::remove_file(&path).map_err(|source| Error::Io {
        context: format!("removing session file {}", path.display()),
        source,
    })?;
    notes.push("session state cleared".to_owned());

    Ok(format!("teardown done — {}", notes.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::{Session, load_from, parse_size, save_new_to, workspace_selector};
    use crate::error::Error;
    use crate::hypr::WorkspaceRef;
    use std::error::Error as StdError;

    fn sample_session() -> Session {
        Session {
            window_address: "0xabc".to_owned(),
            window_title: "App".to_owned(),
            output: "hyprpilot".to_owned(),
            output_created: true,
            workspace: "hyprpilot".to_owned(),
            origin_workspace: Some("3".to_owned()),
            size: [1600, 1000],
            spawned_pid: Some(42),
            initial_user_focus: Some("0xdef".to_owned()),
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
    fn workspace_selector_prefixes_named_workspaces() {
        let named = WorkspaceRef {
            name: "proto".to_owned(),
        };
        let numeric = WorkspaceRef {
            name: "5".to_owned(),
        };
        assert_eq!(workspace_selector(&named), "name:proto");
        assert_eq!(workspace_selector(&numeric), "5");
    }

    #[test]
    fn session_round_trips_through_disk() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        save_new_to(&path, &sample_session())?;
        let loaded = load_from(&path)?;
        assert_eq!(loaded.window_address, "0xabc");
        assert_eq!(loaded.size, [1600, 1000]);
        assert_eq!(loaded.spawned_pid, Some(42));
        assert_eq!(loaded.origin_workspace.as_deref(), Some("3"));
        assert_eq!(loaded.initial_user_focus.as_deref(), Some("0xdef"));
        assert!(loaded.output_created);
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
    fn missing_session_file_is_no_session() {
        let result = load_from(std::path::Path::new("/nonexistent/session.json"));
        assert!(matches!(result, Err(Error::NoSession)));
    }
}
