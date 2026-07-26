//! Thin wrapper around `hyprctl` plus the serde types for its JSON output.

use std::collections::BTreeSet;
use std::process::Command;

use serde::Deserialize;

use crate::error::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceRef {
    pub name: String,
}

/// A monitor's active workspace, whose id `renameworkspace` needs.
#[derive(Debug, Clone, Deserialize)]
pub struct ActiveWorkspace {
    pub id: i64,
    pub name: String,
}

/// The workspace the user is on, with the monitor showing it: `session show`
/// needs both, since a waybar click can leave the focus on an agent desktop's
/// own headless output (§7 of the isolated design).
#[derive(Debug, Clone, Deserialize)]
pub struct FocusedWorkspace {
    pub name: String,
    pub monitor: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub address: String,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub workspace: WorkspaceRef,
    pub floating: bool,
    pub monitor: i64,
    pub class: String,
    pub initial_class: String,
    pub title: String,
    pub initial_title: String,
    pub pid: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: i64,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub transform: u8,
    pub active_workspace: ActiveWorkspace,
    #[serde(deserialize_with = "deserialize_workspace_name")]
    pub special_workspace: String,
}

fn deserialize_workspace_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    WorkspaceRef::deserialize(deserializer).map(|workspace| workspace.name)
}

impl Monitor {
    /// Logical (layout-space) size: physical pixels divided by scale,
    /// swapped for 90°/270° transforms — mirrors Hyprland's `logicalBox()`.
    pub fn logical_size(&self) -> (f64, f64) {
        let w = self.width / self.scale;
        let h = self.height / self.scale;
        if self.transform % 2 == 1 {
            (h, w)
        } else {
            (w, h)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Bounding box of the whole monitor layout, mirroring the mapping used by
/// Hyprland's `CPointerManager::warpAbsolute` for unbound virtual pointers.
pub fn layout_box(monitors: &[Monitor]) -> Result<LayoutBox, Error> {
    let mut iter = monitors.iter();
    let first = iter.next().ok_or_else(|| Error::Tool {
        command: "hyprctl monitors".to_owned(),
        message: "no monitors reported".to_owned(),
    })?;

    let (first_w, first_h) = first.logical_size();
    let (mut min_x, mut min_y) = (first.x, first.y);
    let (mut max_x, mut max_y) = (first.x + first_w, first.y + first_h);
    for monitor in iter {
        let (w, h) = monitor.logical_size();
        min_x = min_x.min(monitor.x);
        min_y = min_y.min(monitor.y);
        max_x = max_x.max(monitor.x + w);
        max_y = max_y.max(monitor.y + h);
    }

    Ok(LayoutBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct Devices {
    pub keyboards: Vec<Keyboard>,
}

/// The XKB fields are what an agent desktop inherits from the host, so its
/// nested compositor types like the user's own keyboard.
#[derive(Debug, Clone, Deserialize)]
pub struct Keyboard {
    pub rules: String,
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: String,
    pub active_keymap: String,
    pub main: bool,
}

/// Which compositor a command is addressed to: the host, or a nested instance
/// identified by its signature. Every `hyprctl` invocation in the crate goes
/// through here, so no code path can silently talk to the wrong compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctl<'a> {
    Host,
    Instance(&'a str),
}

impl Ctl<'_> {
    fn prefix(self) -> Vec<String> {
        match self {
            Self::Host => Vec::new(),
            Self::Instance(signature) => vec!["-i".to_owned(), signature.to_owned()],
        }
    }

    fn label(self, args: &[&str]) -> String {
        match self {
            Self::Host => format!("hyprctl {}", args.join(" ")),
            Self::Instance(signature) => format!("hyprctl -i {signature} {}", args.join(" ")),
        }
    }
}

fn run_on(ctl: Ctl<'_>, args: &[&str]) -> Result<String, Error> {
    let label = ctl.label(args);
    let output = Command::new("hyprctl")
        .args(ctl.prefix())
        .args(args)
        .output()
        .map_err(|source| Error::Io {
            context: format!("running `{label}`"),
            source,
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(Error::Tool {
            command: label,
            message: if stderr.is_empty() { stdout } else { stderr },
        })
    }
}

fn run(args: &[&str]) -> Result<String, Error> {
    run_on(Ctl::Host, args)
}

fn run_json_on<T: serde::de::DeserializeOwned>(ctl: Ctl<'_>, args: &[&str]) -> Result<T, Error> {
    let mut full = args.to_vec();
    full.push("-j");
    let raw = run_on(ctl, &full)?;
    serde_json::from_str(&raw).map_err(|source| Error::Json {
        context: format!("parsing `{}` output", ctl.label(&full)),
        source,
    })
}

/// Runs a dispatcher; Hyprland answers `ok` on success and an error text
/// (sometimes with exit code 0) otherwise.
pub fn dispatch_on(ctl: Ctl<'_>, args: &[&str]) -> Result<(), Error> {
    let mut full = vec!["dispatch"];
    full.extend_from_slice(args);
    let out = run_on(ctl, &full)?;
    if out == "ok" {
        Ok(())
    } else {
        Err(Error::Tool {
            command: ctl.label(&full),
            message: out,
        })
    }
}

pub fn dispatch(args: &[&str]) -> Result<(), Error> {
    dispatch_on(Ctl::Host, args)
}

fn expect_ok(args: &[&str]) -> Result<(), Error> {
    let out = run(args)?;
    if out == "ok" {
        Ok(())
    } else {
        Err(Error::Tool {
            command: format!("hyprctl {}", args.join(" ")),
            message: out,
        })
    }
}

pub fn clients_on(ctl: Ctl<'_>) -> Result<Vec<Client>, Error> {
    run_json_on(ctl, &["clients"])
}

pub fn clients() -> Result<Vec<Client>, Error> {
    clients_on(Ctl::Host)
}

pub fn monitors_on(ctl: Ctl<'_>) -> Result<Vec<Monitor>, Error> {
    run_json_on(ctl, &["monitors"])
}

pub fn monitors() -> Result<Vec<Monitor>, Error> {
    monitors_on(Ctl::Host)
}

/// Every workspace the host knows, by name. The isolated start snapshots this
/// before it creates its output: a workspace that already existed belongs to the
/// user, whatever it holds, so only one the new output brought with it may be
/// renamed (§12.1 of the isolated design).
pub fn workspace_names() -> Result<BTreeSet<String>, Error> {
    let workspaces: Vec<WorkspaceRef> = run_json_on(Ctl::Host, &["workspaces"])?;
    Ok(workspaces
        .into_iter()
        .map(|workspace| workspace.name)
        .collect())
}

pub fn devices() -> Result<Devices, Error> {
    run_json_on(Ctl::Host, &["devices"])
}

pub fn focused_workspace() -> Result<FocusedWorkspace, Error> {
    run_json_on(Ctl::Host, &["activeworkspace"])
}

/// The currently focused window, or `None`.
pub fn active_window_on(ctl: Ctl<'_>) -> Result<Option<Client>, Error> {
    let raw = run_on(ctl, &["activewindow", "-j"])?;
    parse_active_window(&raw)
}

pub fn active_window() -> Result<Option<Client>, Error> {
    active_window_on(Ctl::Host)
}

/// Hyprland prints `Invalid` or an empty object when nothing is focused;
/// only those mean "no active window" — anything else unparseable is a real
/// error (schema drift), not an absent focus.
fn parse_active_window(raw: &str) -> Result<Option<Client>, Error> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "Invalid" || trimmed == "{}" {
        return Ok(None);
    }
    serde_json::from_str::<Client>(trimmed)
        .map(Some)
        .map_err(|source| Error::Json {
            context: "parsing `hyprctl activewindow -j` output".to_owned(),
            source,
        })
}

pub fn cursor_pos_on(ctl: Ctl<'_>) -> Result<(i32, i32), Error> {
    let raw = run_on(ctl, &["cursorpos"])?;
    parse_cursor_pos(&raw).ok_or_else(|| Error::Tool {
        command: ctl.label(&["cursorpos"]),
        message: format!("unparseable output `{raw}`"),
    })
}

pub fn cursor_pos() -> Result<(i32, i32), Error> {
    cursor_pos_on(Ctl::Host)
}

fn parse_cursor_pos(raw: &str) -> Option<(i32, i32)> {
    let (x, y) = raw.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

pub fn output_create_headless(name: &str) -> Result<(), Error> {
    expect_ok(&["output", "create", "headless", name])
}

pub fn output_remove(name: &str) -> Result<(), Error> {
    expect_ok(&["output", "remove", name])
}

/// Mode-set rule for a headless output. The trailing `1` forces scale 1: a
/// headless output otherwise inherits a non-trivial scale from the layout
/// (fact §2.10 of the isolated design), which would skew every coordinate.
pub fn headless_monitor_rule(name: &str, width: u32, height: u32) -> String {
    format!("{name},{width}x{height}@60,auto,1")
}

pub fn keyword_monitor(name: &str, width: u32, height: u32) -> Result<(), Error> {
    expect_ok(&[
        "keyword",
        "monitor",
        &headless_monitor_rule(name, width, height),
    ])
}

/// Mode-set rule for an output that already sits in the layout: its position and
/// scale are handed back unchanged, because resizing an output must neither move
/// it nor rescale it.
pub fn monitor_rule_at(name: &str, width: u32, height: u32, at: (i32, i32), scale: f64) -> String {
    let (x, y) = at;
    format!("{name},{width}x{height}@60,{x}x{y},{scale}")
}

pub fn keyword_monitor_at(
    name: &str,
    width: u32,
    height: u32,
    at: (i32, i32),
    scale: f64,
) -> Result<(), Error> {
    expect_ok(&[
        "keyword",
        "monitor",
        &monitor_rule_at(name, width, height, at, scale),
    ])
}

pub fn keyword_workspace(workspace: &str, monitor: &str) -> Result<(), Error> {
    expect_ok(&[
        "keyword",
        "workspace",
        &format!("{workspace}, monitor:{monitor}"),
    ])
}

pub fn version_on(ctl: Ctl<'_>) -> Result<String, Error> {
    run_on(ctl, &["version"])
}

pub fn version() -> Result<String, Error> {
    version_on(Ctl::Host)
}

#[cfg(test)]
mod tests {
    use super::{
        Client, Devices, Monitor, headless_monitor_rule, layout_box, parse_active_window,
        parse_cursor_pos,
    };
    use std::error::Error;

    const CLIENTS_JSON: &str = include_str!("../fixtures/clients.json");
    const MONITORS_JSON: &str = include_str!("../fixtures/monitors.json");
    const DEVICES_JSON: &str = include_str!("../fixtures/devices.json");

    #[test]
    fn parses_real_clients_fixture() -> Result<(), Box<dyn Error>> {
        let clients: Vec<Client> = serde_json::from_str(CLIENTS_JSON)?;
        assert_eq!(clients.len(), 3, "fixture should hold three clients");

        let proto = clients
            .iter()
            .find(|c| c.title == "PageCairn — Prototype visuel")
            .ok_or("prototype window missing from fixture")?;
        assert_eq!(proto.address, "0x55785d1fb940");
        assert_eq!(proto.at, [5122, 28]);
        assert_eq!(proto.size, [1596, 970]);
        assert_eq!(proto.class, "");
        assert_eq!(proto.initial_class, "");
        assert_eq!(proto.initial_title, "PageCairn — Prototype visuel");
        assert_eq!(proto.workspace.name, "proto");
        assert!(!proto.floating);
        assert_eq!(proto.monitor, 1);
        Ok(())
    }

    #[test]
    fn parses_real_monitors_fixture_and_layout_box() -> Result<(), Box<dyn Error>> {
        let monitors: Vec<Monitor> = serde_json::from_str(MONITORS_JSON)?;
        assert_eq!(monitors.len(), 2, "fixture should hold two monitors");
        assert_eq!(monitors[1].id, 1);
        assert_eq!(monitors[1].name, "headless-ci");
        assert_eq!(monitors[1].active_workspace.name, "proto");
        assert_eq!(monitors[1].special_workspace, "");

        let layout = layout_box(&monitors)?;
        assert_eq!(
            (layout.x, layout.y, layout.width, layout.height),
            (0.0, 0.0, 6720.0, 1440.0)
        );
        Ok(())
    }

    #[test]
    fn layout_box_fails_without_monitors() {
        assert!(layout_box(&[]).is_err());
    }

    #[test]
    fn logical_size_honours_scale_and_transform() -> Result<(), Box<dyn Error>> {
        let monitors: Vec<Monitor> = serde_json::from_str(MONITORS_JSON)?;
        let mut monitor = monitors[0].clone();
        monitor.width = 3840.0;
        monitor.height = 2160.0;
        monitor.scale = 2.0;
        monitor.transform = 0;
        assert_eq!(monitor.logical_size(), (1920.0, 1080.0));
        monitor.transform = 1;
        assert_eq!(monitor.logical_size(), (1080.0, 1920.0));
        Ok(())
    }

    #[test]
    fn parses_devices_fixture_main_keyboard() -> Result<(), Box<dyn Error>> {
        let devices: Devices = serde_json::from_str(DEVICES_JSON)?;
        let main = devices
            .keyboards
            .iter()
            .find(|k| k.main)
            .ok_or("no main keyboard in fixture")?;
        assert_eq!(main.layout, "us");
        assert_eq!(main.variant, "");
        assert_eq!(main.options, "compose:caps");
        assert_eq!(main.rules, "");
        assert_eq!(main.model, "");
        Ok(())
    }

    #[test]
    fn headless_monitor_rule_forces_scale_one() {
        assert_eq!(
            headless_monitor_rule("hyprpilot-alpha", 1920, 1080),
            "hyprpilot-alpha,1920x1080@60,auto,1"
        );
        assert_eq!(
            headless_monitor_rule("hyprpilot", 1600, 1000),
            "hyprpilot,1600x1000@60,auto,1"
        );
    }

    #[test]
    fn parses_cursorpos_output() {
        assert_eq!(parse_cursor_pos("4652, 1066"), Some((4652, 1066)));
        assert_eq!(parse_cursor_pos("-10,20"), Some((-10, 20)));
        assert_eq!(parse_cursor_pos("garbage"), None);
    }

    #[test]
    fn active_window_distinguishes_absent_from_broken() -> Result<(), Box<dyn Error>> {
        assert!(matches!(parse_active_window("Invalid"), Ok(None)));
        assert!(matches!(parse_active_window("{}"), Ok(None)));
        assert!(matches!(parse_active_window(""), Ok(None)));
        assert!(parse_active_window("{\"unexpected\": true}").is_err());

        let clients: Vec<Client> = serde_json::from_str(CLIENTS_JSON)?;
        let raw = serde_json::to_string(&serde_json::json!({
            "address": clients[0].address,
            "at": clients[0].at,
            "size": clients[0].size,
            "workspace": {"id": 1, "name": "1"},
            "floating": clients[0].floating,
            "monitor": clients[0].monitor,
            "class": clients[0].class,
            "initialClass": clients[0].initial_class,
            "title": clients[0].title,
            "initialTitle": clients[0].initial_title,
            "pid": clients[0].pid,
        }))?;
        let parsed = parse_active_window(&raw)?.ok_or("expected a client")?;
        assert_eq!(parsed.address, clients[0].address);
        Ok(())
    }
}
