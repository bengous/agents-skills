//! Screen capture via grim (window-framed by default) and change detection
//! by native PNG pixel diff — replaces fixed sleeps with `wait`.

use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::Error;
use crate::hypr;
use crate::session;

const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(150);

#[derive(Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Image {
    pub fn identical(&self, other: &Self) -> bool {
        self == other
    }
}

pub fn read_png(path: &Path) -> Result<Image, Error> {
    let context = || format!("reading PNG {}", path.display());
    let file = fs::File::open(path).map_err(|source| Error::Io {
        context: context(),
        source,
    })?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info().map_err(|e| Error::Png {
        context: context(),
        message: e.to_string(),
    })?;
    let buffer_size = reader.output_buffer_size().ok_or_else(|| Error::Png {
        context: context(),
        message: "image dimensions overflow the output buffer".to_owned(),
    })?;
    let mut data = vec![0; buffer_size];
    let info = reader.next_frame(&mut data).map_err(|e| Error::Png {
        context: context(),
        message: e.to_string(),
    })?;
    data.truncate(info.buffer_size());
    Ok(Image {
        width: info.width,
        height: info.height,
        data,
    })
}

/// grim region string for the window, clamped to its output's logical box.
pub fn crop_geometry(
    at: [i32; 2],
    size: [i32; 2],
    monitor: &hypr::Monitor,
) -> Result<String, Error> {
    let (mon_width, mon_height) = monitor.logical_size();
    let (left, top) = (monitor.x, monitor.y);
    let (right, bottom) = (monitor.x + mon_width, monitor.y + mon_height);

    let x0 = f64::from(at[0]).max(left);
    let y0 = f64::from(at[1]).max(top);
    let x1 = f64::from(at[0] + size[0]).min(right);
    let y1 = f64::from(at[1] + size[1]).min(bottom);

    if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
        return Err(Error::Invalid {
            what: "capture geometry",
            value: format!("window at {at:?} size {size:?}"),
            hint: format!("window does not intersect output {}", monitor.name),
        });
    }
    Ok(format!("{x0:.0},{y0:.0} {:.0}x{:.0}", x1 - x0, y1 - y0))
}

fn grim(args: &[&str]) -> Result<(), Error> {
    let output = Command::new("grim")
        .args(args)
        .output()
        .map_err(|source| Error::Io {
            context: format!("running `grim {}`", args.join(" ")),
            source,
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Tool {
            command: format!("grim {}", args.join(" ")),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

/// Window (or `--full` output) capture context for the active session.
struct Frame {
    output_name: String,
    geometry: Option<String>,
}

impl Frame {
    fn for_session(full: bool) -> Result<Self, Error> {
        let (session, window) = session::current_window()?;
        let monitor = session::find_output(&session.output)?.ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("session output {} no longer exists", session.output),
        })?;
        // grim captures screen regions: if the parked workspace is not the
        // active one on the headless output, the capture would silently show
        // the wallpaper instead of the window.
        if monitor.active_workspace.name != session.workspace {
            return Err(Error::Tool {
                command: "hyprctl monitors".to_owned(),
                message: format!(
                    "workspace {} is not active on output {} (active: {}) — captures would \
                     not show the window",
                    session.workspace, session.output, monitor.active_workspace.name
                ),
            });
        }
        let geometry = if full {
            None
        } else {
            Some(crop_geometry(window.at, window.size, &monitor)?)
        };
        Ok(Self {
            output_name: monitor.name,
            geometry,
        })
    }

    fn capture(&self, dest: &Path) -> Result<(), Error> {
        let dest_str = dest.to_string_lossy();
        self.geometry.as_ref().map_or_else(
            || grim(&["-o", &self.output_name, &dest_str]),
            |geometry| grim(&["-g", geometry, &dest_str]),
        )
    }
}

fn next_shot_name(dir: &Path) -> u32 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 1;
    };
    let mut max = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(n) = name
            .strip_prefix("shot-")
            .and_then(|rest| rest.strip_suffix(".png"))
            .and_then(|num| num.parse::<u32>().ok())
        {
            max = max.max(n);
        }
    }
    max + 1
}

pub fn shot(name: Option<&str>, full: bool, out_dir: Option<&Path>) -> Result<String, Error> {
    let frame = Frame::for_session(full)?;

    let dir = out_dir.map_or_else(
        || session::runtime_dir().map(|dir| dir.join("shots")),
        |dir| Ok(dir.to_path_buf()),
    )?;
    fs::create_dir_all(&dir).map_err(|source| Error::Io {
        context: format!("creating {}", dir.display()),
        source,
    })?;

    let has_png_extension = |name: &str| {
        Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    };
    let file_name = match name {
        Some(name) if has_png_extension(name) => name.to_owned(),
        Some(name) => format!("{name}.png"),
        None => format!("shot-{:04}.png", next_shot_name(&dir)),
    };
    let dest = dir.join(file_name);
    frame.capture(&dest)?;

    let absolute = dest.canonicalize().map_err(|source| Error::Io {
        context: format!("resolving {}", dest.display()),
        source,
    })?;
    Ok(absolute.to_string_lossy().into_owned())
}

#[derive(Debug)]
pub enum WaitMode {
    /// Two consecutive identical captures.
    Stable,
    /// A capture that differs from the reference PNG.
    ChangedFrom(PathBuf),
}

pub fn parse_timeout(raw: &str) -> Result<Duration, Error> {
    let invalid = || Error::Invalid {
        what: "timeout",
        value: raw.to_owned(),
        hint: "expected e.g. `5s`, `2.5s` or `800ms`".to_owned(),
    };
    let trimmed = raw.trim();
    if let Some(ms) = trimmed.strip_suffix("ms") {
        let value: u64 = ms.trim().parse().map_err(|_| invalid())?;
        if value == 0 {
            return Err(invalid());
        }
        return Ok(Duration::from_millis(value));
    }
    let seconds = trimmed.strip_suffix('s').unwrap_or(trimmed);
    let value: f64 = seconds.trim().parse().map_err(|_| invalid())?;
    if !value.is_finite() || value <= 0.0 {
        return Err(invalid());
    }
    Ok(Duration::from_secs_f64(value))
}

pub fn wait(mode: &WaitMode, timeout: Duration) -> Result<String, Error> {
    let frame = Frame::for_session(false)?;
    let dir = session::runtime_dir()?;
    fs::create_dir_all(&dir).map_err(|source| Error::Io {
        context: format!("creating {}", dir.display()),
        source,
    })?;
    let poll_paths = [dir.join("wait-a.png"), dir.join("wait-b.png")];

    let started = Instant::now();
    let result = wait_loop(&frame, mode, timeout, &poll_paths, started);
    for path in &poll_paths {
        // Best-effort scratch cleanup; the wait result is what matters.
        let _ = fs::remove_file(path);
    }
    result
}

fn wait_loop(
    frame: &Frame,
    mode: &WaitMode,
    timeout: Duration,
    poll_paths: &[PathBuf; 2],
    started: Instant,
) -> Result<String, Error> {
    let reference = match mode {
        WaitMode::Stable => None,
        WaitMode::ChangedFrom(path) => Some(read_png(path)?),
    };

    let mut previous: Option<Image> = None;
    let mut captures: u32 = 0;
    loop {
        if captures > 0 {
            thread::sleep(WAIT_POLL_INTERVAL);
        }
        if started.elapsed() > timeout {
            let what = match mode {
                WaitMode::Stable => "a stable frame".to_owned(),
                WaitMode::ChangedFrom(path) => {
                    format!("a change from {}", path.display())
                }
            };
            return Err(Error::Timeout {
                what,
                after_ms: timeout.as_millis(),
            });
        }

        let path = &poll_paths[(captures % 2) as usize];
        frame.capture(path)?;
        captures += 1;
        let current = read_png(path)?;

        if let Some(reference) = &reference {
            if !current.identical(reference) {
                return Ok(format!(
                    "changed after {}ms ({captures} capture(s))",
                    started.elapsed().as_millis()
                ));
            }
        } else {
            if previous.is_some_and(|p| p.identical(&current)) {
                return Ok(format!(
                    "stable after {}ms ({captures} capture(s))",
                    started.elapsed().as_millis()
                ));
            }
            previous = Some(current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Image, crop_geometry, parse_timeout, read_png};
    use crate::hypr::Monitor;
    use std::error::Error as StdError;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    fn monitor(x: f64, y: f64, width: f64, height: f64) -> Result<Monitor, Box<dyn StdError>> {
        let json = format!(
            r#"{{"id": 1, "name": "headless-ci", "width": {width}, "height": {height},
                "x": {x}, "y": {y}, "scale": 1.0, "transform": 0, "focused": false,
                "activeWorkspace": {{"id": -1, "name": "proto"}}, "disabled": false}}"#
        );
        Ok(serde_json::from_str(&json)?)
    }

    fn write_png(
        path: &Path,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<(), Box<dyn StdError>> {
        let file = fs::File::create(path)?;
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(data)?;
        Ok(())
    }

    #[test]
    fn crop_geometry_matches_window_inside_output() -> Result<(), Box<dyn StdError>> {
        let monitor = monitor(5120.0, 0.0, 1600.0, 1000.0)?;
        let geometry = crop_geometry([5122, 28], [1596, 970], &monitor)?;
        assert_eq!(geometry, "5122,28 1596x970");
        Ok(())
    }

    #[test]
    fn crop_geometry_clamps_overflow_to_output() -> Result<(), Box<dyn StdError>> {
        let monitor = monitor(5120.0, 0.0, 1600.0, 1000.0)?;
        let geometry = crop_geometry([5000, -50], [2000, 3000], &monitor)?;
        assert_eq!(geometry, "5120,0 1600x1000");
        Ok(())
    }

    #[test]
    fn crop_geometry_rejects_disjoint_window() -> Result<(), Box<dyn StdError>> {
        let monitor = monitor(5120.0, 0.0, 1600.0, 1000.0)?;
        assert!(crop_geometry([0, 0], [100, 100], &monitor).is_err());
        Ok(())
    }

    #[test]
    fn timeout_parses_seconds_and_millis() {
        assert_eq!(parse_timeout("5s").ok(), Some(Duration::from_secs(5)));
        assert_eq!(
            parse_timeout("2.5s").ok(),
            Some(Duration::from_millis(2500))
        );
        assert_eq!(
            parse_timeout("800ms").ok(),
            Some(Duration::from_millis(800))
        );
        assert_eq!(parse_timeout("3").ok(), Some(Duration::from_secs(3)));
        assert!(parse_timeout("-1s").is_err());
        assert!(parse_timeout("0ms").is_err());
        assert!(parse_timeout("0").is_err());
        assert!(parse_timeout("fast").is_err());
    }

    #[test]
    fn png_round_trip_and_diff() -> Result<(), Box<dyn StdError>> {
        let dir = tempfile::tempdir()?;
        let a_path = dir.path().join("a.png");
        let b_path = dir.path().join("b.png");
        let c_path = dir.path().join("c.png");

        let mut pixels = vec![0u8; 4 * 4 * 3];
        write_png(&a_path, 4, 4, &pixels)?;
        write_png(&b_path, 4, 4, &pixels)?;
        pixels[0] = 255;
        write_png(&c_path, 4, 4, &pixels)?;

        let a = read_png(&a_path)?;
        let b = read_png(&b_path)?;
        let c = read_png(&c_path)?;
        assert_eq!(a.width, 4);
        assert_eq!(a.height, 4);
        assert!(a.identical(&b), "same pixels must compare identical");
        assert!(!a.identical(&c), "one changed pixel must be detected");
        Ok(())
    }

    #[test]
    fn images_with_different_dimensions_differ() {
        let a = Image {
            width: 2,
            height: 2,
            data: vec![0; 12],
        };
        let b = Image {
            width: 4,
            height: 1,
            data: vec![0; 12],
        };
        assert!(!a.identical(&b));
    }
}
