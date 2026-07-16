//! Thin wrapper around `hyprctl` plus the serde types for its JSON output.

use std::process::Command;

use serde::Deserialize;

use crate::error::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceRef {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Client {
    pub address: String,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub workspace: WorkspaceRef,
    pub class: String,
    pub title: String,
    pub pid: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub transform: u8,
    pub active_workspace: WorkspaceRef,
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

#[derive(Debug, Clone, Deserialize)]
pub struct Keyboard {
    pub layout: String,
    pub active_keymap: String,
    pub main: bool,
}

fn run(args: &[&str]) -> Result<String, Error> {
    let output = Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|source| Error::Io {
            context: format!("running `hyprctl {}`", args.join(" ")),
            source,
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(Error::Tool {
            command: format!("hyprctl {}", args.join(" ")),
            message: if stderr.is_empty() { stdout } else { stderr },
        })
    }
}

fn run_json<T: serde::de::DeserializeOwned>(args: &[&str]) -> Result<T, Error> {
    let mut full = args.to_vec();
    full.push("-j");
    let raw = run(&full)?;
    serde_json::from_str(&raw).map_err(|source| Error::Json {
        context: format!("parsing `hyprctl {} -j` output", args.join(" ")),
        source,
    })
}

/// Runs a dispatcher; Hyprland answers `ok` on success and an error text
/// (sometimes with exit code 0) otherwise.
pub fn dispatch(args: &[&str]) -> Result<(), Error> {
    let mut full = vec!["dispatch"];
    full.extend_from_slice(args);
    let out = run(&full)?;
    if out == "ok" {
        Ok(())
    } else {
        Err(Error::Tool {
            command: format!("hyprctl dispatch {}", args.join(" ")),
            message: out,
        })
    }
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

pub fn clients() -> Result<Vec<Client>, Error> {
    run_json(&["clients"])
}

pub fn monitors() -> Result<Vec<Monitor>, Error> {
    run_json(&["monitors"])
}

pub fn devices() -> Result<Devices, Error> {
    run_json(&["devices"])
}

/// The currently focused window, or `None`.
pub fn active_window() -> Result<Option<Client>, Error> {
    let raw = run(&["activewindow", "-j"])?;
    parse_active_window(&raw)
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

pub fn cursor_pos() -> Result<(i32, i32), Error> {
    let raw = run(&["cursorpos"])?;
    parse_cursor_pos(&raw).ok_or_else(|| Error::Tool {
        command: "hyprctl cursorpos".to_owned(),
        message: format!("unparseable output `{raw}`"),
    })
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

pub fn keyword_monitor(name: &str, width: u32, height: u32) -> Result<(), Error> {
    expect_ok(&[
        "keyword",
        "monitor",
        &format!("{name},{width}x{height}@60,auto,1"),
    ])
}

pub fn version() -> Result<String, Error> {
    run(&["version"])
}

#[cfg(test)]
mod tests {
    use super::{Client, Devices, Monitor, layout_box, parse_active_window, parse_cursor_pos};
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
        assert_eq!(proto.workspace.name, "proto");
        Ok(())
    }

    #[test]
    fn parses_real_monitors_fixture_and_layout_box() -> Result<(), Box<dyn Error>> {
        let monitors: Vec<Monitor> = serde_json::from_str(MONITORS_JSON)?;
        assert_eq!(monitors.len(), 2, "fixture should hold two monitors");
        assert_eq!(monitors[1].name, "headless-ci");
        assert_eq!(monitors[1].active_workspace.name, "proto");

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
        Ok(())
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
            "class": clients[0].class,
            "title": clients[0].title,
            "pid": clients[0].pid,
        }))?;
        let parsed = parse_active_window(&raw)?.ok_or("expected a client")?;
        assert_eq!(parsed.address, clients[0].address);
        Ok(())
    }
}
