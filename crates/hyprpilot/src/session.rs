//! Session state: one named session per agent, `shared` (drive the user's own
//! windows on a headless output) or `isolated` (a whole nested agent desktop).
//! State lives in `$XDG_RUNTIME_DIR/hyprpilot/sessions/<name>/session.json`;
//! creating it with `create_new` is the per-name lock, and it is written
//! **before** any compositor side effect so a failed start stays recoverable
//! via `teardown`.

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

/// Shared mode drives the user's windows on this single output; it is a
/// singleton, unlike the per-session `hyprpilot-<name>` outputs of isolated
/// mode.
pub const OUTPUT_NAME: &str = "hyprpilot";
pub const WORKSPACE_NAME: &str = "hyprpilot";
const PARKING_WORKSPACE_NAME: &str = "special:hyprpilot-parked";
pub const SCHEMA_VERSION: u32 = 3;
pub const DEFAULT_SESSION_NAME: &str = "default";
const SESSION_ENV: &str = "HYPRPILOT_SESSION";
const SESSION_NAME_MAX: usize = 32;
const WINDOW_APPEAR_TIMEOUT: Duration = Duration::from_secs(15);
const WINDOW_CLOSE_TIMEOUT: Duration = Duration::from_secs(3);
const WINDOW_PLACE_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(200);
const VERIFIED_PLACEMENT_READS: u8 = 2;

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub schema_version: u32,
    pub name: String,
    /// Flattened so the mode payload sits at the top level of `session.json`,
    /// tagged by `"mode"`.
    #[serde(flatten)]
    pub state: ModeState,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ModeState {
    Shared(Shared),
    Isolated(Isolated),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Shared,
    Isolated,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shared {
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

/// An agent desktop: a nested Hyprland whose console window lives on the
/// active workspace of a host headless output.
#[derive(Debug, Serialize, Deserialize)]
pub struct Isolated {
    /// Host headless output, `hyprpilot-<name>`.
    pub output: String,
    /// Workspace renamed on that output, `agent-<name>`.
    pub workspace: String,
    pub size: [u32; 2],
    /// True while the console window sits on the user's workspace
    /// (`session show`).
    pub shown: bool,
    pub instance: Instance,
}

/// The nested compositor is acquired after the output, so the state has to
/// describe a session whose instance does not exist yet: `teardown` must be
/// able to clean up either stage.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "lowercase")]
pub enum Instance {
    Pending,
    Live {
        /// `HYPRLAND_INSTANCE_SIGNATURE` of the nested compositor.
        signature: String,
        /// Wayland socket the nested compositor serves.
        wayland_display: String,
        pid: u32,
        /// Host-side address of the nested compositor's console window
        /// (class `aquamarine`).
        console_address: String,
    },
}

/// A command whose isolated-mode implementation lands in a later slice of
/// `hyprpilot-isolated-slice-plan.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pending {
    command: &'static str,
    slice: &'static str,
}

impl Pending {
    pub const START: Self = Self::new("session start --isolated", "S3–S6");
    pub const TEARDOWN: Self = Self::new("teardown", "S5");
    pub const KEY: Self = Self::new("key", "S7");
    pub const TYPE: Self = Self::new("type", "S7");
    pub const CLICK: Self = Self::new("click", "S7");
    pub const SCROLL: Self = Self::new("scroll", "S7");
    pub const SHOT: Self = Self::new("shot", "S8");
    pub const WAIT: Self = Self::new("wait", "S8");
    pub const TARGET: Self = Self::new("target", "S9");
    pub const WINDOWS: Self = Self::new("windows", "S9");
    pub const STATUS: Self = Self::new("status", "S11");

    const fn new(command: &'static str, slice: &'static str) -> Self {
        Self { command, slice }
    }

    pub fn error(self) -> Error {
        Error::IsolatedPending {
            command: self.command,
            slice: self.slice,
        }
    }
}

impl Session {
    pub fn mode(&self) -> Mode {
        match self.state {
            ModeState::Shared(_) => Mode::Shared,
            ModeState::Isolated(_) => Mode::Isolated,
        }
    }

    /// Routes by mode: an isolated session never falls through to the shared
    /// code path, which would mutate the user's desktop.
    pub fn shared(&self, pending: Pending) -> Result<&Shared, Error> {
        self.shared_or(|| pending.error())
    }

    fn shared_mut(&mut self, pending: Pending) -> Result<&mut Shared, Error> {
        self.shared_mut_or(|| pending.error())
    }

    fn shared_or(&self, error: impl FnOnce() -> Error) -> Result<&Shared, Error> {
        match &self.state {
            ModeState::Shared(shared) => Ok(shared),
            ModeState::Isolated(_) => Err(error()),
        }
    }

    fn shared_mut_or(&mut self, error: impl FnOnce() -> Error) -> Result<&mut Shared, Error> {
        match &mut self.state {
            ModeState::Shared(shared) => Ok(shared),
            ModeState::Isolated(_) => Err(error()),
        }
    }
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

#[derive(Debug, Default)]
pub struct Criteria<'a> {
    pub address: Option<&'a str>,
    pub title: Option<&'a str>,
    pub class: Option<&'a str>,
    pub pid: Option<i32>,
}

#[derive(Debug)]
pub enum Resolution<'a> {
    Unique(&'a hypr::Client),
    None,
    Ambiguous(Vec<&'a hypr::Client>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetMode {
    Switch,
    Adopt,
}

#[derive(Debug)]
enum TargetLookup<'a> {
    Ready {
        client: &'a hypr::Client,
        mode: TargetMode,
    },
    Retry,
}

fn matches_criteria(client: &hypr::Client, criteria: &Criteria<'_>) -> bool {
    criteria
        .address
        .is_none_or(|address| client.address == address)
        && criteria.title.is_none_or(|title| client.title == title)
        && criteria.class.is_none_or(|class| client.class == class)
        && criteria.pid.is_none_or(|pid| client.pid == pid)
}

fn resolution(matches: Vec<&hypr::Client>) -> Resolution<'_> {
    match matches.len() {
        0 => Resolution::None,
        1 => Resolution::Unique(matches[0]),
        _ => Resolution::Ambiguous(matches),
    }
}

pub fn resolve<'a>(clients: &'a [hypr::Client], criteria: &Criteria<'_>) -> Resolution<'a> {
    resolution(
        clients
            .iter()
            .filter(|client| matches_criteria(client, criteria))
            .collect(),
    )
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

/// Read before the full parse so an old file fails with the version it holds
/// instead of a serde field error. Both fields are optional: a pre-v3 file has
/// neither, and reporting that is the point.
#[derive(Deserialize)]
struct SessionHeader {
    schema_version: Option<u32>,
    mode: Option<Mode>,
}

#[derive(Debug, Deserialize)]
struct LegacySession {
    window_address: String,
    output: String,
    output_created: bool,
    origin_workspace: Option<String>,
    spawned_pid: Option<u32>,
}

/// State at the old unnamed location, readable by `teardown` only.
enum PreV3Session {
    V2(Shared),
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

pub fn sessions_dir() -> Result<PathBuf, Error> {
    Ok(runtime_dir()?.join("sessions"))
}

pub fn session_dir(name: &str) -> Result<PathBuf, Error> {
    Ok(sessions_dir()?.join(name))
}

pub fn session_path(name: &str) -> Result<PathBuf, Error> {
    Ok(session_dir(name)?.join("session.json"))
}

/// Where v2 and the unversioned format kept their single session.
fn pre_v3_session_path() -> Result<PathBuf, Error> {
    Ok(runtime_dir()?.join("session.json"))
}

/// `--session NAME`, else `$HYPRPILOT_SESSION`, else `default`. The name ends
/// up in a filesystem path, so it is validated on every command, not just at
/// start.
pub fn resolve_name(flag: Option<&str>) -> Result<String, Error> {
    let from_env = env::var_os(SESSION_ENV).map(|value| value.to_string_lossy().into_owned());
    resolve_name_from(flag, from_env.as_deref())
}

fn resolve_name_from(flag: Option<&str>, from_env: Option<&str>) -> Result<String, Error> {
    let (name, source) = match (flag, from_env) {
        (Some(name), _) => (name, "--session"),
        (None, Some(name)) => (name, SESSION_ENV),
        (None, None) => (DEFAULT_SESSION_NAME, "the default"),
    };
    validate_name(name, source)?;
    Ok(name.to_owned())
}

fn validate_name(name: &str, source: &str) -> Result<(), Error> {
    let allowed = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-';
    if !name.is_empty() && name.len() <= SESSION_NAME_MAX && name.bytes().all(allowed) {
        return Ok(());
    }
    Err(Error::Invalid {
        what: "session name",
        value: name.to_owned(),
        hint: format!("expected [a-z0-9-]{{1,{SESSION_NAME_MAX}}} (from {source})"),
    })
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

fn unsupported_version(path: &Path, found: Option<u32>) -> Error {
    Error::UnsupportedSessionVersion {
        path: path.to_path_buf(),
        found,
    }
}

fn load_from(path: &Path) -> Result<Session, Error> {
    let raw = read_from(path)?;
    let header: SessionHeader = parse_json(&raw, path)?;
    if header.schema_version != Some(SCHEMA_VERSION) {
        return Err(unsupported_version(path, header.schema_version));
    }
    let session: Session = parse_json(&raw, path)?;
    if let ModeState::Shared(shared) = &session.state {
        check_primary(shared, path)?;
    }
    Ok(session)
}

fn check_primary(shared: &Shared, path: &Path) -> Result<(), Error> {
    if shared
        .windows
        .iter()
        .filter(|window| window.address == shared.primary_address)
        .count()
        == 1
    {
        return Ok(());
    }
    Err(Error::CorruptSession {
        path: path.to_path_buf(),
        message: "primary_address must identify exactly one tracked window".to_owned(),
    })
}

/// Refuses to run on state left behind by an older build: the v3 layout moved,
/// so a caller that only checked the new path would silently ignore a live
/// pre-v3 session still parked on the shared output.
fn refuse_pre_v3_state_at(path: &Path) -> Result<(), Error> {
    match fs::read_to_string(path) {
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Io {
            context: format!("reading session file {}", path.display()),
            source,
        }),
        // Best effort on the version: an unparseable file reports none, and the
        // message says so.
        Ok(raw) => Err(unsupported_version(
            path,
            serde_json::from_str::<SessionHeader>(&raw)
                .ok()
                .and_then(|header| header.schema_version),
        )),
    }
}

fn refuse_pre_v3_state() -> Result<(), Error> {
    refuse_pre_v3_state_at(&pre_v3_session_path()?)
}

// Pre-v3 parsing stays separate so no command except teardown can accept the
// old location and its formats.
fn load_pre_v3_from(path: &Path) -> Result<PreV3Session, Error> {
    let raw = read_from(path)?;
    let corrupt = |error: Error| Error::CorruptSession {
        path: path.to_path_buf(),
        message: error.to_string(),
    };
    let value: serde_json::Value = parse_json(&raw, path).map_err(&corrupt)?;
    let Some(version) = value.get("schema_version") else {
        return parse_json_value(value, path)
            .map(PreV3Session::Legacy)
            .map_err(corrupt);
    };
    if version.as_u64() != Some(2) {
        return Err(unsupported_version(
            path,
            version.as_u64().and_then(|found| u32::try_from(found).ok()),
        ));
    }
    let shared: Shared = parse_json_value(value, path).map_err(&corrupt)?;
    check_primary(&shared, path)?;
    Ok(PreV3Session::V2(shared))
}

/// The shared output is a singleton, so a second shared session is refused
/// whatever its name.
fn find_shared_session_in(dir: &Path, exclude: &str) -> Result<Option<String>, Error> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::Io {
                context: format!("listing {}", dir.display()),
                source,
            });
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            context: format!("listing {}", dir.display()),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == exclude {
            continue;
        }
        let path = entry.path().join("session.json");
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(Error::Io {
                    context: format!("reading session file {}", path.display()),
                    source,
                });
            }
        };
        let header: SessionHeader = parse_json(&raw, &path)?;
        if header.schema_version != Some(SCHEMA_VERSION) {
            return Err(unsupported_version(&path, header.schema_version));
        }
        match header.mode {
            Some(Mode::Shared) => return Ok(Some(name)),
            Some(Mode::Isolated) => {}
            None => {
                return Err(Error::CorruptSession {
                    path,
                    message: "missing mode".to_owned(),
                });
            }
        }
    }
    Ok(None)
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
            return Err(Error::SessionExists {
                name: session.name.clone(),
                path: path.to_path_buf(),
            });
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

pub fn load(name: &str) -> Result<Session, Error> {
    match load_from(&session_path(name)?) {
        Err(Error::NoSession) => {
            refuse_pre_v3_state()?;
            Err(Error::NoSession)
        }
        result => result,
    }
}

/// The shared session's active window as Hyprland currently sees it.
pub fn current_window(
    name: &str,
    pending: Pending,
) -> Result<(CurrentSession, hypr::Client), Error> {
    let session = load(name)?;
    let shared = session.shared(pending)?;
    let clients = hypr::clients()?;
    let window = clients
        .into_iter()
        .find(|c| c.address == shared.active_address)
        .ok_or_else(|| Error::WindowGone(shared.active_address.clone()))?;
    Ok((
        CurrentSession {
            output: shared.output.clone(),
            workspace: shared.active_workspace.clone(),
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

fn criteria_label(criteria: &Criteria<'_>) -> String {
    let mut parts = Vec::new();
    if let Some(address) = criteria.address {
        parts.push(format!("address `{address}`"));
    }
    if let Some(title) = criteria.title {
        parts.push(format!("title `{title}`"));
    }
    if let Some(class) = criteria.class {
        parts.push(format!("class `{class}`"));
    }
    if let Some(pid) = criteria.pid {
        parts.push(format!("pid `{pid}`"));
    }
    parts.join(" and ")
}

fn target_criteria_label(criteria: &Criteria<'_>, untracked: bool) -> String {
    let criteria = criteria_label(criteria);
    match (criteria.is_empty(), untracked) {
        (true, true) => "untracked windows".to_owned(),
        (false, true) => format!("{criteria} among untracked windows"),
        _ => criteria,
    }
}

fn ambiguous_error_with_label(criteria: String, candidates: Vec<&hypr::Client>) -> Error {
    let candidates = candidates
        .into_iter()
        .map(|client| {
            serde_json::json!({
                "address": client.address,
                "class": client.class,
                "initial_class": client.initial_class,
                "title": client.title,
                "initial_title": client.initial_title,
                "pid": client.pid,
                "workspace": client.workspace.name,
                "at": client.at,
                "size": client.size,
                "floating": client.floating,
                "monitor": client.monitor,
            })
        })
        .collect();
    Error::WindowAmbiguous {
        criteria,
        candidates: serde_json::Value::Array(candidates),
    }
}

fn ambiguous_error(criteria: &Criteria<'_>, candidates: Vec<&hypr::Client>) -> Error {
    ambiguous_error_with_label(criteria_label(criteria), candidates)
}

fn resolve_target<'a>(
    clients: &'a [hypr::Client],
    session: &Shared,
    criteria: &Criteria<'_>,
    untracked: bool,
) -> Resolution<'a> {
    resolution(
        clients
            .iter()
            .filter(|client| {
                matches_criteria(client, criteria)
                    && (!untracked
                        || !session
                            .windows
                            .iter()
                            .any(|window| window.address == client.address))
            })
            .collect(),
    )
}

fn target_lookup<'a>(
    clients: &'a [hypr::Client],
    session: &Shared,
    criteria: &Criteria<'_>,
    untracked: bool,
    wait: bool,
) -> Result<TargetLookup<'a>, Error> {
    match resolve_target(clients, session, criteria, untracked) {
        Resolution::Unique(client) => Ok(TargetLookup::Ready {
            client,
            mode: if session
                .windows
                .iter()
                .any(|window| window.address == client.address)
            {
                TargetMode::Switch
            } else {
                TargetMode::Adopt
            },
        }),
        Resolution::None if wait => Ok(TargetLookup::Retry),
        Resolution::None => Err(Error::WindowNotFound(target_criteria_label(
            criteria, untracked,
        ))),
        Resolution::Ambiguous(candidates) => Err(ambiguous_error_with_label(
            target_criteria_label(criteria, untracked),
            candidates,
        )),
    }
}

fn target_disposition(
    address: &str,
    mode: TargetMode,
    requested: Option<Disposition>,
) -> Result<Disposition, Error> {
    if mode == TargetMode::Switch
        && let Some(disposition) = requested
    {
        let value = match disposition {
            Disposition::Restore => "restore",
            Disposition::Close => "close",
        };
        return Err(Error::Invalid {
            what: "target option",
            value: format!("--on-teardown {value}"),
            hint: format!("window {address} is already tracked; omit --on-teardown when switching"),
        });
    }
    Ok(requested.unwrap_or(Disposition::Restore))
}

fn find_window(criteria: &Criteria<'_>) -> Result<Option<hypr::Client>, Error> {
    let clients = hypr::clients()?;
    match resolve(&clients, criteria) {
        Resolution::Unique(client) => Ok(Some(client.clone())),
        Resolution::None => Ok(None),
        Resolution::Ambiguous(candidates) => Err(ambiguous_error(criteria, candidates)),
    }
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

fn wait_for_window(criteria: &Criteria<'_>) -> Result<Option<hypr::Client>, Error> {
    let deadline = Instant::now() + WINDOW_APPEAR_TIMEOUT;
    loop {
        if let Some(window) = find_window(criteria)? {
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

fn effective_output_size(output: &hypr::Monitor) -> Result<[u32; 2], Error> {
    let dimension = |value: f64, field: &str| {
        let value = value.to_string().parse::<u32>().map_err(|_| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("output {} reports invalid {field} {value}", output.name),
        })?;
        if value == 0 {
            return Err(Error::Tool {
                command: "hyprctl monitors".to_owned(),
                message: format!("output {} reports zero {field}", output.name),
            });
        }
        Ok(value)
    };
    Ok([
        dimension(output.width, "width")?,
        dimension(output.height, "height")?,
    ])
}

fn monitor_rule(output: &hypr::Monitor, width: u32, height: u32) -> Result<String, Error> {
    let x = exact_layout_integer(output.x, "x", &output.name)?;
    let y = exact_layout_integer(output.y, "y", &output.name)?;
    Ok(format!(
        "{},{width}x{height}@60,{x}x{y},{}",
        output.name, output.scale
    ))
}

fn resize_monitor(output: &hypr::Monitor, width: u32, height: u32) -> Result<(), Error> {
    let rule = monitor_rule(output, width, height)?;
    let command = format!("hyprctl keyword monitor {rule}");
    let result = Command::new("hyprctl")
        .args(["keyword", "monitor", &rule])
        .output()
        .map_err(|source| Error::Io {
            context: format!("running `{command}`"),
            source,
        })?;
    let stdout = String::from_utf8_lossy(&result.stdout).trim().to_owned();
    if result.status.success() && stdout == "ok" {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&result.stderr).trim().to_owned();
    Err(Error::Tool {
        command,
        message: if stderr.is_empty() { stdout } else { stderr },
    })
}

fn resize_has_applied(
    previous_size: [u32; 2],
    requested_size: [u32; 2],
    effective_size: [u32; 2],
) -> bool {
    requested_size == previous_size || effective_size != previous_size
}

fn wait_for_effective_resize(
    output_name: &str,
    previous_size: [u32; 2],
    requested_size: [u32; 2],
) -> Result<[u32; 2], Error> {
    let deadline = Instant::now() + WINDOW_PLACE_TIMEOUT;
    loop {
        let output = find_output(output_name)?.ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("session output {output_name} is missing after resize"),
        })?;
        let effective_size = effective_output_size(&output)?;
        if resize_has_applied(previous_size, requested_size, effective_size) {
            return Ok(effective_size);
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout {
                what: format!(
                    "output {output_name} to apply resize {}x{} (last effective size: {}x{})",
                    requested_size[0], requested_size[1], effective_size[0], effective_size[1]
                ),
                after_ms: WINDOW_PLACE_TIMEOUT.as_millis(),
            });
        }
        thread::sleep(POLL_INTERVAL);
    }
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
    place_session_window(address, output_name, workspace_name)
}

fn place_session_window(
    address: &str,
    output_name: &str,
    workspace_name: &str,
) -> Result<Option<Placement>, Error> {
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

fn target_layout_is_verified(
    session: &Shared,
    clients: &[hypr::Client],
    monitors: &[hypr::Monitor],
) -> Result<bool, Error> {
    let output = monitors
        .iter()
        .find(|monitor| monitor.name == session.output)
        .ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("session output {} is missing", session.output),
        })?;
    if output.active_workspace.name != session.active_workspace
        || !output.special_workspace.is_empty()
        || monitors
            .iter()
            .any(|monitor| monitor.special_workspace == session.parking_workspace)
    {
        return Ok(false);
    }

    let active = clients
        .iter()
        .find(|client| client.address == session.active_address)
        .ok_or_else(|| Error::WindowGone(session.active_address.clone()))?;
    if active.workspace.name != session.active_workspace || active.monitor != output.id {
        return Ok(false);
    }

    for tracked in &session.windows {
        if tracked.address == session.active_address {
            continue;
        }
        if let Some(client) = clients
            .iter()
            .find(|client| client.address == tracked.address)
            && client.workspace.name != session.parking_workspace
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn wait_for_target_layout(session: &Shared) -> Result<(), Error> {
    let deadline = Instant::now() + WINDOW_PLACE_TIMEOUT;
    loop {
        let clients = hypr::clients()?;
        let monitors = hypr::monitors()?;
        if target_layout_is_verified(session, &clients, &monitors)? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout {
                what: format!(
                    "one active tracked window {} on output {} and every other tracked window on {}",
                    session.active_address, session.output, session.parking_workspace
                ),
                after_ms: WINDOW_PLACE_TIMEOUT.as_millis(),
            });
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn activate_persisted_target(session: &Shared) -> Result<(), Error> {
    hypr::keyword_workspace(&session.parking_workspace, &session.output)?;

    let clients = hypr::clients()?;
    for tracked in &session.windows {
        if tracked.address == session.active_address {
            continue;
        }
        if let Some(client) = clients
            .iter()
            .find(|client| client.address == tracked.address)
            && client.workspace.name != session.parking_workspace
        {
            hypr::dispatch(&[
                "movetoworkspacesilent",
                &format!("{},address:{}", session.parking_workspace, tracked.address),
            ])?;
        }
    }

    let target = hypr::clients()?
        .into_iter()
        .find(|client| client.address == session.active_address)
        .ok_or_else(|| Error::WindowGone(session.active_address.clone()))?;
    if target.workspace.name != session.active_workspace {
        hypr::dispatch(&[
            "movetoworkspacesilent",
            &format!(
                "name:{},address:{}",
                session.active_workspace, session.active_address
            ),
        ])?;
    }

    place_active_target(session)
}

fn place_active_target(session: &Shared) -> Result<(), Error> {
    if matches!(
        place_session_window(
            &session.active_address,
            &session.output,
            &session.active_workspace,
        )?,
        Some(Placement::Oversized(_, _))
    ) {
        let _ = writeln!(
            std::io::stderr(),
            "hyprpilot: warning: window {} is larger than output {}; \
             use `hyprpilot session resize`",
            session.active_address,
            session.output
        );
    }
    wait_for_target_layout(session)
}

fn persist_target_before_activation(
    path: &Path,
    session: &mut Session,
    client: &hypr::Client,
    mode: TargetMode,
    disposition: Disposition,
) -> Result<(), Error> {
    if mode == TargetMode::Adopt {
        session
            .shared_mut(Pending::TARGET)?
            .windows
            .push(TrackedWindow {
                address: client.address.clone(),
                title_at_adoption: client.title.clone(),
                origin_workspace: client.workspace.name.clone(),
                origin_at: client.at,
                origin_size: client.size,
                origin_floating: client.floating,
                teardown: disposition,
            });
        save_over(path, session)?;
    }

    session
        .shared_mut(Pending::TARGET)?
        .active_address
        .clone_from(&client.address);
    save_over(path, session)
}

pub fn target(
    name: &str,
    criteria: &Criteria<'_>,
    untracked: bool,
    wait: Option<Duration>,
    on_teardown: Option<Disposition>,
) -> Result<String, Error> {
    if criteria.address.is_none()
        && criteria.title.is_none()
        && criteria.class.is_none()
        && criteria.pid.is_none()
        && !untracked
    {
        return Err(Error::Invalid {
            what: "target selector",
            value: "(none)".to_owned(),
            hint: "pass --address, --match-title, --match-class, --pid and/or --untracked"
                .to_owned(),
        });
    }

    let path = session_path(name)?;
    let mut session = load(name)?;
    // Routed before the first compositor read, so an isolated session fails on
    // its own terms instead of on a host query that means nothing to it.
    session.shared(Pending::TARGET)?;
    let started = Instant::now();
    let (client, mode) = loop {
        let clients = hypr::clients()?;
        let shared = session.shared(Pending::TARGET)?;
        match target_lookup(&clients, shared, criteria, untracked, wait.is_some())? {
            TargetLookup::Ready { client, mode } => break (client.clone(), mode),
            TargetLookup::Retry => {
                let timeout = wait.ok_or_else(|| {
                    Error::WindowNotFound(target_criteria_label(criteria, untracked))
                })?;
                let elapsed = started.elapsed();
                if elapsed >= timeout {
                    return Err(Error::Timeout {
                        what: format!(
                            "a window matching {}",
                            target_criteria_label(criteria, untracked)
                        ),
                        after_ms: timeout.as_millis(),
                    });
                }
                thread::sleep(POLL_INTERVAL.min(timeout.saturating_sub(elapsed)));
            }
        }
    };
    let disposition = target_disposition(&client.address, mode, on_teardown)?;

    // Adoption (when needed) and the active address are persisted before the
    // first compositor command. Activation only accepts that resulting state.
    persist_target_before_activation(&path, &mut session, &client, mode, disposition)?;
    activate_persisted_target(session.shared(Pending::TARGET)?)?;

    Ok(format!(
        "target active — {} window {} (`{}`)",
        match mode {
            TargetMode::Switch => "switched to",
            TargetMode::Adopt => "adopted",
        },
        client.address,
        client.title
    ))
}

/// Out of scope for this cycle, by spec: resizing an agent desktop means
/// mode-setting the headless output and resizing the nested console.
fn resize_unsupported() -> Error {
    Error::IsolatedUnsupported {
        command: "session resize",
        hint: "an agent desktop keeps the size it was started with; run \
               `teardown` then `session start --isolated --size WxH`",
    }
}

pub fn resize(name: &str, size: &str) -> Result<String, Error> {
    let (width, height) = parse_size(size)?;
    let requested_size = [width, height];
    let path = session_path(name)?;
    let mut session = load(name)?;
    let shared = session.shared_or(resize_unsupported)?;
    let output = find_output(&shared.output)?.ok_or_else(|| Error::Tool {
        command: "hyprctl monitors".to_owned(),
        message: format!("session output {} is missing", shared.output),
    })?;
    let previous_size = effective_output_size(&output)?;
    let output_name = shared.output.clone();

    resize_monitor(&output, width, height)?;

    let effective_size = wait_for_effective_resize(&output_name, previous_size, requested_size)?;
    session.shared_mut_or(resize_unsupported)?.size = effective_size;
    save_over(&path, &session)?;
    let shared = session.shared_or(resize_unsupported)?;
    place_active_target(shared)?;

    Ok(format!(
        "session resized — output {} is {}x{}, window {} repositioned",
        shared.output, effective_size[0], effective_size[1], shared.active_address
    ))
}

/// Everything that must be refused before an app is spawned: state left by an
/// older build, a claim already held under this name, and a second shared
/// session. The atomic `save_new_to` below stays the authoritative lock.
fn claim_preflight(name: &str, path: &Path) -> Result<(), Error> {
    refuse_pre_v3_state()?;
    if path.exists() {
        return Err(Error::SessionExists {
            name: name.to_owned(),
            path: path.to_path_buf(),
        });
    }
    if let Some(other) = find_shared_session_in(&sessions_dir()?, name)? {
        return Err(Error::SharedSessionExists { name: other });
    }
    Ok(())
}

pub fn start(
    name: &str,
    isolated: bool,
    app: Option<&str>,
    match_title: Option<&str>,
    match_class: Option<&str>,
    size: &str,
) -> Result<String, Error> {
    // Refused before the session is claimed and before any compositor call, so
    // no agent desktop is half-created while the slices land.
    if isolated {
        return Err(Pending::START.error());
    }
    if match_title.is_none() && match_class.is_none() {
        return Err(Error::Invalid {
            what: "match criteria",
            value: "(none)".to_owned(),
            hint: "pass --match-title and/or --match-class".to_owned(),
        });
    }
    let (width, height) = parse_size(size)?;
    let path = session_path(name)?;
    claim_preflight(name, &path)?;

    // Captured before any side effect, so status/teardown can reason about
    // what the user had.
    let initial_user_focus = hypr::active_window()?.map(|w| w.address);

    let criteria = Criteria {
        address: None,
        title: match_title,
        class: match_class,
        pid: None,
    };
    let criteria_description = criteria_label(&criteria);
    let mut spawned_pid = None;
    let window = if let Some(window) = find_window(&criteria)? {
        window
    } else {
        let Some(command) = app else {
            return Err(Error::WindowNotFound(format!(
                "{criteria_description} — pass --app to launch it"
            )));
        };
        let pid = spawn_app(command)?;
        spawned_pid = Some(pid);
        match wait_for_window(&criteria) {
            Ok(Some(window)) => window,
            Ok(None) => {
                let _ = kill_process_group(pid);
                return Err(Error::WindowNotFound(format!(
                    "{criteria_description} after launching `{command}` ({}s timeout) — process \
                     killed",
                    WINDOW_APPEAR_TIMEOUT.as_secs()
                )));
            }
            Err(error) => {
                let _ = kill_process_group(pid);
                return Err(error);
            }
        }
    };

    let output_created = find_output(OUTPUT_NAME)?.is_none();
    let teardown = spawned_pid.map_or(Disposition::Restore, |_| Disposition::Close);
    let session = Session {
        schema_version: SCHEMA_VERSION,
        name: name.to_owned(),
        state: ModeState::Shared(Shared {
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
        }),
    };
    // Lock + persist before touching the compositor: if anything below
    // fails, `hyprpilot teardown` can still clean up from this state.
    if let Err(error) = save_new_to(&path, &session) {
        if let (Error::SessionExists { .. }, Some(pid)) = (&error, spawned_pid) {
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
             use `hyprpilot session resize`",
            window.address
        );
    }

    Ok(format!(
        "session `{name}` ready — window {} (`{}`) parked on output {OUTPUT_NAME} \
         ({width}x{height}), workspace {WORKSPACE_NAME}",
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
    session: &Shared,
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

fn teardown_shared(session: &Shared, kill: bool, close: bool) -> Result<Vec<String>, Error> {
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

enum StateLocation {
    /// v3: the whole `sessions/<name>/` directory goes away.
    Session(PathBuf),
    /// Pre-v3: a single file at the old, unnamed location.
    PreV3(PathBuf),
}

fn clear_state(location: &StateLocation) -> Result<(), Error> {
    let (path, result) = match location {
        StateLocation::Session(dir) => (dir, fs::remove_dir_all(dir)),
        StateLocation::PreV3(file) => (file, fs::remove_file(file)),
    };
    result.map_err(|source| Error::Io {
        context: format!("removing session state {}", path.display()),
        source,
    })
}

fn finish_teardown(
    location: &StateLocation,
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

    clear_state(location)?;
    notes.push("session state cleared".to_owned());
    Ok(format!("teardown done — {}", notes.join(", ")))
}

pub fn teardown(name: &str, kill: bool, close: bool) -> Result<String, Error> {
    match load_from(&session_path(name)?) {
        Ok(session) => {
            let shared = session.shared(Pending::TEARDOWN)?;
            let notes = teardown_shared(shared, kill, close)?;
            finish_teardown(
                &StateLocation::Session(session_dir(name)?),
                &shared.output,
                shared.output_created,
                notes,
            )
        }
        // The v3 layout moved, so teardown stays the one command that can still
        // clean up what an older build left at the old location.
        Err(Error::NoSession) => teardown_pre_v3(kill, close),
        Err(error) => Err(error),
    }
}

fn teardown_pre_v3(kill: bool, close: bool) -> Result<String, Error> {
    let path = pre_v3_session_path()?;
    let state = match load_pre_v3_from(&path) {
        Ok(state) => state,
        Err(Error::NoSession) => return sweep_orphan_output(),
        Err(error) => return Err(error),
    };
    let location = StateLocation::PreV3(path.clone());
    let migrated = format!("cleaned pre-v3 state at {}", path.display());
    match state {
        PreV3Session::V2(shared) => {
            let mut notes = vec![migrated];
            notes.extend(teardown_shared(&shared, kill, close)?);
            finish_teardown(&location, &shared.output, shared.output_created, notes)
        }
        PreV3Session::Legacy(legacy) => {
            let mut notes = vec![migrated];
            notes.extend(teardown_legacy(&legacy, kill, close)?);
            finish_teardown(&location, &legacy.output, legacy.output_created, notes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Criteria, DEFAULT_SESSION_NAME, Disposition, Instance, Isolated, Mode, ModeState, Pending,
        Placement, PreV3Session, Rect, Resolution, SCHEMA_VERSION, Session, Shared, TargetLookup,
        TargetMode, TrackedWindow, WindowAction, ambiguous_error, effective_output_size,
        ensure_output_empty_for_sweep, find_shared_session_in, load_from, load_pre_v3_from,
        monitor_rule, parse_size, persist_target_before_activation, place, refuse_pre_v3_state_at,
        resize_has_applied, resize_unsupported, resolve, resolve_name_from, save_new_to, save_over,
        target_disposition, target_layout_is_verified, target_lookup, teardown_plan,
        workspace_selector,
    };
    use crate::error::Error;
    use crate::hypr::{Client, Monitor, WorkspaceRef};
    use std::error::Error as StdError;

    const MONITORS_JSON: &str = include_str!("../fixtures/monitors.json");
    const AMBIGUOUS_CLIENTS_JSON: &str = include_str!("../fixtures/clients-ambiguous.json");
    const SWEEP_OCCUPIED_JSON: &str =
        include_str!("../fixtures/sweep-clients-output-occupied.json");
    const SWEEP_EMPTY_JSON: &str = include_str!("../fixtures/sweep-clients-output-empty.json");
    const SWEEP_MINUS_ONE_JSON: &str =
        include_str!("../fixtures/sweep-clients-monitor-minus-one.json");

    fn sample_shared() -> Shared {
        Shared {
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

    fn named_session(name: &str) -> Session {
        Session {
            schema_version: SCHEMA_VERSION,
            name: name.to_owned(),
            state: ModeState::Shared(sample_shared()),
        }
    }

    fn sample_session() -> Session {
        named_session(DEFAULT_SESSION_NAME)
    }

    fn sample_isolated(name: &str, instance: Instance) -> Session {
        Session {
            schema_version: SCHEMA_VERSION,
            name: name.to_owned(),
            state: ModeState::Isolated(Isolated {
                output: format!("hyprpilot-{name}"),
                workspace: format!("agent-{name}"),
                size: [1920, 1080],
                shown: false,
                instance,
            }),
        }
    }

    fn live_instance() -> Instance {
        Instance::Live {
            signature: "abcdef_1730000000".to_owned(),
            wayland_display: "wayland-2".to_owned(),
            pid: 4242,
            console_address: "0xc0ff33".to_owned(),
        }
    }

    fn shared_of(session: &Session) -> Result<&Shared, Box<dyn StdError>> {
        Ok(session.shared(Pending::STATUS)?)
    }

    fn matching_clients() -> Result<Vec<Client>, serde_json::Error> {
        serde_json::from_str(AMBIGUOUS_CLIENTS_JSON)
    }

    fn tracked_client(client: &Client) -> TrackedWindow {
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

    #[test]
    fn resolve_returns_none_when_no_client_matches() -> Result<(), Box<dyn StdError>> {
        let clients = matching_clients()?;
        let criteria = Criteria {
            title: Some("Missing title"),
            ..Criteria::default()
        };

        assert!(matches!(resolve(&clients, &criteria), Resolution::None));
        Ok(())
    }

    #[test]
    fn resolve_returns_unique_client_for_one_exact_match() -> Result<(), Box<dyn StdError>> {
        let clients = matching_clients()?;
        let criteria = Criteria {
            title: Some("Unique title"),
            ..Criteria::default()
        };
        let Resolution::Unique(client) = resolve(&clients, &criteria) else {
            return Err("expected one matching client".into());
        };

        assert_eq!(client.address, "0xddd");
        Ok(())
    }

    #[test]
    fn resolve_returns_every_ambiguous_exact_match() -> Result<(), Box<dyn StdError>> {
        let clients = matching_clients()?;
        let criteria = Criteria {
            title: Some("Shared title"),
            ..Criteria::default()
        };
        let Resolution::Ambiguous(candidates) = resolve(&clients, &criteria) else {
            return Err("expected ambiguous clients".into());
        };

        assert_eq!(
            candidates
                .iter()
                .map(|client| client.address.as_str())
                .collect::<Vec<_>>(),
            vec!["0xaaa", "0xbbb", "0xccc"]
        );
        Ok(())
    }

    #[test]
    fn resolve_combines_title_class_and_pid_with_and() -> Result<(), Box<dyn StdError>> {
        let clients = matching_clients()?;
        let criteria = Criteria {
            address: Some("0xbbb"),
            title: Some("Shared title"),
            class: Some("shared.class"),
            pid: Some(102),
        };
        let Resolution::Unique(client) = resolve(&clients, &criteria) else {
            return Err("combined criteria did not resolve uniquely".into());
        };

        assert_eq!(client.address, "0xbbb");
        Ok(())
    }

    #[test]
    fn target_resolution_matrix_covers_tracking_count_wait_and_untracked()
    -> Result<(), Box<dyn StdError>> {
        let all_clients = matching_clients()?;
        let criteria = Criteria {
            title: Some("Shared title"),
            ..Criteria::default()
        };

        for tracked in [false, true] {
            for count in [0, 1, 3] {
                for wait in [false, true] {
                    for untracked in [false, true] {
                        let clients = &all_clients[..count];
                        let mut session = sample_shared();
                        if tracked {
                            session.windows.extend(clients.iter().map(tracked_client));
                        }
                        let effective_count = if tracked && untracked { 0 } else { count };
                        let result = target_lookup(clients, &session, &criteria, untracked, wait);
                        let label = format!(
                            "tracked={tracked}, count={count}, wait={wait}, untracked={untracked}"
                        );

                        match effective_count {
                            0 if wait => {
                                assert!(matches!(result, Ok(TargetLookup::Retry)), "{label}");
                            }
                            0 => {
                                assert!(matches!(result, Err(Error::WindowNotFound(_))), "{label}");
                            }
                            1 => {
                                let Ok(TargetLookup::Ready { mode, .. }) = result else {
                                    return Err(format!("{label}: expected one target").into());
                                };
                                assert_eq!(
                                    mode,
                                    if tracked {
                                        TargetMode::Switch
                                    } else {
                                        TargetMode::Adopt
                                    },
                                    "{label}"
                                );
                            }
                            _ => {
                                let Err(Error::WindowAmbiguous { candidates, .. }) = result else {
                                    return Err(format!("{label}: expected ambiguity").into());
                                };
                                assert_eq!(
                                    candidates.as_array().map(Vec::len),
                                    Some(effective_count),
                                    "{label}"
                                );
                            }
                        }
                    }
                }
            }
        }
        let Err(Error::WindowAmbiguous {
            criteria,
            candidates,
        }) = target_lookup(
            &all_clients,
            &sample_shared(),
            &Criteria::default(),
            true,
            false,
        )
        else {
            return Err("--untracked alone did not select all untracked clients".into());
        };
        assert_eq!(criteria, "untracked windows");
        assert_eq!(candidates.as_array().map(Vec::len), Some(4));
        Ok(())
    }

    #[test]
    fn target_rejects_disposition_for_switch_and_defaults_adoption_to_restore()
    -> Result<(), Box<dyn StdError>> {
        assert_eq!(
            target_disposition("0xabc", TargetMode::Switch, None)?,
            Disposition::Restore
        );
        let error = target_disposition("0xabc", TargetMode::Switch, Some(Disposition::Close))
            .err()
            .ok_or("tracked target accepted --on-teardown")?;
        assert!(error.to_string().contains("already tracked"));
        assert!(error.to_string().contains("omit --on-teardown"));
        assert_eq!(
            target_disposition("0xdef", TargetMode::Adopt, None)?,
            Disposition::Restore
        );
        assert_eq!(
            target_disposition("0xdef", TargetMode::Adopt, Some(Disposition::Close))?,
            Disposition::Close
        );
        Ok(())
    }

    #[test]
    fn target_persistence_finishes_before_activation() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        let mut session = sample_session();
        save_new_to(&path, &session)?;
        let client = matching_clients()?
            .into_iter()
            .find(|client| client.address == "0xddd")
            .ok_or("target fixture missing")?;

        persist_target_before_activation(
            &path,
            &mut session,
            &client,
            TargetMode::Adopt,
            Disposition::Close,
        )?;

        let persisted = load_from(&path)?;
        let persisted = shared_of(&persisted)?;
        assert_eq!(persisted.active_address, "0xddd");
        assert_eq!(persisted.windows.len(), 2);
        assert_eq!(persisted.windows[1].address, "0xddd");
        assert_eq!(persisted.windows[1].origin_workspace, "4");
        assert_eq!(persisted.windows[1].origin_at, [70, 80]);
        assert_eq!(persisted.windows[1].origin_size, [800, 600]);
        assert!(!persisted.windows[1].origin_floating);
        assert_eq!(persisted.windows[1].teardown, Disposition::Close);
        Ok(())
    }

    #[test]
    fn target_layout_requires_only_the_active_window_visible() -> Result<(), Box<dyn StdError>> {
        let mut clients = matching_clients()?;
        clients.truncate(2);
        clients[0].workspace.name = "proto".to_owned();
        clients[0].monitor = 1;
        clients[1].workspace.name = "special:hyprpilot-parked".to_owned();
        clients[1].monitor = 1;
        let mut monitors: Vec<Monitor> = serde_json::from_str(MONITORS_JSON)?;
        let mut session = sample_shared();
        session.output = "headless-ci".to_owned();
        session.active_workspace = "proto".to_owned();
        session.active_address.clone_from(&clients[0].address);
        session.primary_address.clone_from(&clients[0].address);
        session.windows = clients.iter().map(tracked_client).collect();

        assert!(target_layout_is_verified(&session, &clients, &monitors)?);

        clients[1].workspace.name = "proto".to_owned();
        assert!(!target_layout_is_verified(&session, &clients, &monitors)?);

        clients[1].workspace.name = "special:hyprpilot-parked".to_owned();
        monitors[0].special_workspace = "special:hyprpilot-parked".to_owned();
        assert!(!target_layout_is_verified(&session, &clients, &monitors)?);

        monitors[0].special_workspace.clear();
        clients.pop();
        assert!(target_layout_is_verified(&session, &clients, &monitors)?);
        Ok(())
    }

    #[test]
    fn ambiguous_error_ends_with_machine_readable_candidates() -> Result<(), Box<dyn StdError>> {
        let clients = matching_clients()?;
        let criteria = Criteria {
            address: None,
            title: Some("Shared title"),
            class: Some("shared.class"),
            pid: None,
        };
        let Resolution::Ambiguous(candidates) = resolve(&clients, &criteria) else {
            return Err("expected ambiguous clients".into());
        };
        let message = ambiguous_error(&criteria, candidates).to_string();
        let last_line = message.lines().last().ok_or("missing candidate line")?;
        let candidates: Vec<serde_json::Value> = serde_json::from_str(last_line)?;

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate["address"].as_str())
                .collect::<Vec<_>>(),
            vec![Some("0xaaa"), Some("0xbbb")]
        );
        Ok(())
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
    fn monitor_rule_preserves_position_and_scale() -> Result<(), Box<dyn StdError>> {
        let monitors: Vec<Monitor> = serde_json::from_str(MONITORS_JSON)?;

        assert_eq!(
            monitor_rule(&monitors[1], 1200, 800)?,
            "headless-ci,1200x800@60,5120x0,1"
        );
        Ok(())
    }

    #[test]
    fn effective_output_size_uses_compositor_dimensions() -> Result<(), Box<dyn StdError>> {
        let monitors: Vec<Monitor> = serde_json::from_str(MONITORS_JSON)?;

        assert_eq!(effective_output_size(&monitors[1])?, [1600, 1000]);
        Ok(())
    }

    #[test]
    fn resize_rejects_stale_pre_keyword_dimensions() {
        assert!(!resize_has_applied([300, 200], [1200, 800], [300, 200]));
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
        let spawned = sample_shared();
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

        let mut attached = sample_shared();
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
        let mut session = sample_shared();
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
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
        assert_eq!(loaded.name, DEFAULT_SESSION_NAME);
        assert_eq!(loaded.mode(), Mode::Shared);
        let loaded = shared_of(&loaded)?;
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
    fn isolated_session_round_trips_at_both_instance_stages() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        for (label, instance) in [("pending", Instance::Pending), ("live", live_instance())] {
            let path = dir.path().join(label).join("session.json");
            save_new_to(&path, &sample_isolated("alpha", instance))?;

            let loaded = load_from(&path)?;
            assert_eq!(loaded.schema_version, SCHEMA_VERSION, "{label}");
            assert_eq!(loaded.name, "alpha", "{label}");
            assert_eq!(loaded.mode(), Mode::Isolated, "{label}");
            let ModeState::Isolated(isolated) = &loaded.state else {
                return Err(format!("{label}: isolated state loaded as shared").into());
            };
            assert_eq!(isolated.output, "hyprpilot-alpha", "{label}");
            assert_eq!(isolated.workspace, "agent-alpha", "{label}");
            assert_eq!(isolated.size, [1920, 1080], "{label}");
            assert!(!isolated.shown, "{label}");
            match (&isolated.instance, label) {
                (Instance::Pending, "pending") => {}
                (
                    Instance::Live {
                        signature,
                        wayland_display,
                        pid,
                        console_address,
                    },
                    "live",
                ) => {
                    assert_eq!(signature, "abcdef_1730000000");
                    assert_eq!(wayland_display, "wayland-2");
                    assert_eq!(*pid, 4242);
                    assert_eq!(console_address, "0xc0ff33");
                }
                (instance, label) => {
                    return Err(format!("{label}: unexpected stage {instance:?}").into());
                }
            }
        }
        Ok(())
    }

    #[test]
    fn session_json_keeps_the_mode_payload_flat_and_tagged() -> Result<(), Box<dyn StdError>> {
        let shared = serde_json::to_value(sample_session())?;
        assert_eq!(shared["mode"], serde_json::json!("shared"));
        assert_eq!(shared["output"], serde_json::json!("hyprpilot"));
        assert!(shared.get("state").is_none());

        let isolated = serde_json::to_value(sample_isolated("alpha", live_instance()))?;
        assert_eq!(isolated["mode"], serde_json::json!("isolated"));
        assert_eq!(isolated["output"], serde_json::json!("hyprpilot-alpha"));
        assert_eq!(isolated["instance"]["stage"], serde_json::json!("live"));
        assert_eq!(isolated["instance"]["pid"], serde_json::json!(4242));
        Ok(())
    }

    #[test]
    fn session_overwrite_replaces_atomically() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        save_new_to(&path, &sample_session())?;
        let mut updated = sample_session();
        updated.shared_mut(Pending::TARGET)?.active_address = "0xdef".to_owned();

        save_over(&path, &updated)?;

        assert_eq!(shared_of(&load_from(&path)?)?.active_address, "0xdef");
        assert!(
            !dir.path()
                .join(format!("session.json.{}.tmp", std::process::id()))
                .exists()
        );
        Ok(())
    }

    #[test]
    fn claim_is_atomic_per_session_name() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = |name: &str| dir.path().join("sessions").join(name).join("session.json");
        save_new_to(&path("alpha"), &named_session("alpha"))?;
        save_new_to(&path("beta"), &named_session("beta"))?;

        assert_eq!(load_from(&path("alpha"))?.name, "alpha");
        assert_eq!(load_from(&path("beta"))?.name, "beta");

        let error = save_new_to(&path("alpha"), &named_session("alpha"))
            .err()
            .ok_or("a second claim on the same name was accepted")?;
        assert!(matches!(&error, Error::SessionExists { .. }));
        let message = error.to_string();
        assert!(message.contains("session `alpha`"), "{message}");
        assert!(message.contains("--session alpha teardown"), "{message}");
        Ok(())
    }

    #[test]
    fn shared_session_singleton_is_refused_under_any_other_name() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let sessions = dir.path().join("sessions");
        let path = |name: &str| sessions.join(name).join("session.json");
        assert_eq!(find_shared_session_in(&sessions, "alpha")?, None);

        save_new_to(&path("alpha"), &named_session("alpha"))?;
        save_new_to(&path("beta"), &sample_isolated("beta", Instance::Pending))?;
        std::fs::create_dir_all(sessions.join("stale"))?;

        // The shared output is a singleton: only the session that holds it may
        // start.
        assert_eq!(
            find_shared_session_in(&sessions, "beta")?,
            Some("alpha".to_owned())
        );
        assert_eq!(
            find_shared_session_in(&sessions, "gamma")?.as_deref(),
            Some("alpha")
        );
        assert_eq!(find_shared_session_in(&sessions, "alpha")?, None);

        let error = Error::SharedSessionExists {
            name: "alpha".to_owned(),
        }
        .to_string();
        assert!(error.contains("singleton"), "{error}");
        assert!(error.contains("--session alpha teardown"), "{error}");
        assert!(error.contains("--isolated"), "{error}");
        Ok(())
    }

    #[test]
    fn pre_v3_state_is_refused_outside_teardown_and_read_by_teardown()
    -> Result<(), Box<dyn StdError>> {
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

        let error = refuse_pre_v3_state_at(&path)
            .err()
            .ok_or("unversioned state was accepted")?;
        assert!(matches!(
            &error,
            Error::UnsupportedSessionVersion { found: None, .. }
        ));
        let message = error.to_string();
        assert!(message.contains("unversioned"), "{message}");
        assert!(message.contains("hyprpilot teardown"), "{message}");
        assert!(matches!(
            load_from(&path),
            Err(Error::UnsupportedSessionVersion { found: None, .. })
        ));

        let PreV3Session::Legacy(legacy) = load_pre_v3_from(&path)? else {
            return Err("legacy file loaded as v2".into());
        };
        assert_eq!(legacy.window_address, "0xabc");
        assert_eq!(legacy.origin_workspace.as_deref(), Some("3"));
        assert!(legacy.attached());
        Ok(())
    }

    #[test]
    fn v2_state_reports_found_version_expected_version_and_exit_command()
    -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        let mut value = serde_json::to_value(sample_shared())?;
        value["schema_version"] = serde_json::json!(2);
        std::fs::write(&path, serde_json::to_vec_pretty(&value)?)?;

        let error = load_from(&path)
            .err()
            .ok_or("a v2 state was accepted by a v3 build")?;
        assert!(matches!(
            &error,
            Error::UnsupportedSessionVersion { found: Some(2), .. }
        ));
        let message = error.to_string();
        assert!(message.contains("schema version 2"), "{message}");
        assert!(message.contains("expects 3"), "{message}");
        assert!(message.contains("hyprpilot teardown"), "{message}");
        assert!(message.contains("no output was removed"), "{message}");
        assert!(message.contains("hyprpilot windows"), "{message}");

        // Teardown is the one command that still reads it, from the old
        // location only.
        assert!(matches!(load_pre_v3_from(&path)?, PreV3Session::V2(_)));
        assert!(refuse_pre_v3_state_at(&path).is_err());
        Ok(())
    }

    #[test]
    fn unknown_session_version_is_rejected_explicitly() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("session.json");
        let mut value = serde_json::to_value(sample_session())?;
        value["schema_version"] = serde_json::json!(4);
        std::fs::write(&path, serde_json::to_vec_pretty(&value)?)?;

        let Err(error) = load_from(&path) else {
            return Err("unknown schema was accepted".into());
        };
        assert!(matches!(
            &error,
            Error::UnsupportedSessionVersion { found: Some(4), .. }
        ));
        assert!(error.to_string().contains("no output was removed"));
        assert!(error.to_string().contains("hyprpilot windows"));
        assert!(matches!(
            load_pre_v3_from(&path),
            Err(Error::UnsupportedSessionVersion { found: Some(4), .. })
        ));
        Ok(())
    }

    #[test]
    fn session_name_is_validated_against_the_path_safe_alphabet() {
        for name in ["a", "default", "agent-1", "a".repeat(32).as_str()] {
            assert!(
                resolve_name_from(Some(name), None).is_ok(),
                "rejected valid name {name}"
            );
        }
        for name in [
            "",
            "Agent",
            "agent_1",
            "agent.1",
            "../etc",
            "a/b",
            "a".repeat(33).as_str(),
        ] {
            let error = resolve_name_from(Some(name), None)
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            assert!(error.contains("session name"), "accepted {name:?}");
            assert!(error.contains("--session"), "{error}");
        }
    }

    #[test]
    fn session_name_resolution_prefers_flag_then_env_then_default() -> Result<(), Box<dyn StdError>>
    {
        assert_eq!(resolve_name_from(Some("alpha"), Some("beta"))?, "alpha");
        assert_eq!(resolve_name_from(None, Some("beta"))?, "beta");
        assert_eq!(resolve_name_from(None, None)?, DEFAULT_SESSION_NAME);
        let error = resolve_name_from(None, Some(""))
            .err()
            .ok_or("an empty HYPRPILOT_SESSION was accepted")?
            .to_string();
        assert!(error.contains("HYPRPILOT_SESSION"), "{error}");
        Ok(())
    }

    #[test]
    fn isolated_sessions_route_every_pending_command_to_its_slice() -> Result<(), Box<dyn StdError>>
    {
        let pending = [
            Pending::START,
            Pending::TEARDOWN,
            Pending::KEY,
            Pending::TYPE,
            Pending::CLICK,
            Pending::SCROLL,
            Pending::SHOT,
            Pending::WAIT,
            Pending::TARGET,
            Pending::WINDOWS,
            Pending::STATUS,
        ];
        let session = sample_isolated("alpha", live_instance());
        for pending in pending {
            let error = session
                .shared(pending)
                .err()
                .ok_or_else(|| format!("`{}` fell through to the shared path", pending.command))?
                .to_string();
            assert!(error.contains(pending.command), "{error}");
            assert!(error.contains(pending.slice), "{error}");
            assert!(error.contains("not implemented"), "{error}");
            assert!(error.contains("no compositor state was touched"), "{error}");
        }
        // The same accessor hands the payload over in shared mode.
        assert_eq!(
            sample_session().shared(Pending::STATUS)?.output,
            "hyprpilot"
        );
        Ok(())
    }

    #[test]
    fn isolated_resize_is_refused_as_out_of_cycle() -> Result<(), Box<dyn StdError>> {
        let error = sample_isolated("alpha", live_instance())
            .shared_or(resize_unsupported)
            .err()
            .ok_or("isolated resize fell through to the shared path")?
            .to_string();
        assert!(error.contains("session resize"), "{error}");
        assert!(error.contains("not supported"), "{error}");
        assert!(error.contains("--isolated --size"), "{error}");
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

        let error = load_pre_v3_from(&path)
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
