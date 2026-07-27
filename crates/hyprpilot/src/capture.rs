//! Screen capture via grim (window-framed by default) and change detection
//! by native PNG pixel diff — replaces fixed sleeps with `wait`.

use std::fs;
use std::io::{BufReader, Read as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::Error;
use crate::isolated;
use crate::session;
use crate::{hypr, session::ModeState};

const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(150);
/// Every capture, shared or isolated, runs under a bounded deadline — grim
/// blocks for ever on a compositor that stopped answering screencopy, and no
/// `wait` loop timeout can interrupt that.
const CAPTURE_ESCALATION: session::Escalation = session::Escalation {
    polite: Duration::from_secs(5),
    term: Duration::from_secs(1),
    kill: Duration::from_secs(1),
    poll: Duration::from_millis(25),
};

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

/// How a bounded child ended.
enum Ran {
    Exited {
        status: ExitStatus,
        stderr: String,
    },
    /// Never produced its frame and was killed; `signal` is the escalation that
    /// ended it.
    Killed {
        signal: &'static str,
        after: Duration,
    },
}

/// Runs a child under a bounded deadline: `SIGTERM` when the deadline passes,
/// `SIGKILL` one grace period later. `try_wait` keeps this free of any new
/// dependency, and a child that survives even `SIGKILL` is reported rather than
/// waited on for ever.
fn run_bounded(child: &mut Child, ladder: session::Escalation, label: &str) -> Result<Ran, Error> {
    let started = Instant::now();
    let mut sent: Option<&'static str> = None;
    loop {
        let exited = child.try_wait().map_err(|source| Error::Io {
            context: format!("waiting for `{label}`"),
            source,
        })?;
        if let Some(status) = exited {
            // A child that finished in the same instant it was signalled still
            // wrote its frame, so only a failed status counts as killed.
            return Ok(match sent {
                Some(signal) if !status.success() => Ran::Killed {
                    signal,
                    after: started.elapsed(),
                },
                _ => Ran::Exited {
                    status,
                    stderr: read_stderr(child, label)?,
                },
            });
        }
        match ladder.step(started.elapsed()) {
            session::Step::Signal(signal) if sent != Some(signal) => {
                let _ = session::signal_process(child.id(), signal);
                sent = Some(signal);
            }
            session::Step::Wait | session::Step::Signal(_) => {}
            session::Step::GiveUp => {
                return Ok(Ran::Killed {
                    signal: sent.unwrap_or("KILL"),
                    after: started.elapsed(),
                });
            }
        }
        thread::sleep(ladder.poll);
    }
}

/// Read only once the child is gone, so a full pipe can never deadlock the poll
/// loop above.
fn read_stderr(child: &mut Child, label: &str) -> Result<String, Error> {
    let Some(mut stderr) = child.stderr.take() else {
        return Ok(String::new());
    };
    let mut message = String::new();
    stderr
        .read_to_string(&mut message)
        .map_err(|source| Error::Io {
            context: format!("reading stderr of `{label}`"),
            source,
        })?;
    Ok(message)
}

fn grim(display: Option<&str>, args: &[&str], blocked: &Blocked) -> Result<(), Error> {
    let label = format!("grim {}", args.join(" "));
    let mut command = Command::new("grim");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(display) = display {
        // The agent desktop's own socket: grim then talks to the nested
        // compositor, in the nested layout's coordinates.
        command.env("WAYLAND_DISPLAY", display);
    }
    let mut child = command.spawn().map_err(|source| Error::Io {
        context: format!("running `{label}`"),
        source,
    })?;

    match run_bounded(&mut child, CAPTURE_ESCALATION, &label)? {
        Ran::Exited { status, .. } if status.success() => Ok(()),
        Ran::Exited { stderr, .. } => Err(Error::Tool {
            command: label,
            message: stderr.trim().to_owned(),
        }),
        Ran::Killed { signal, after } => Err(Error::CaptureTimeout {
            command: label,
            after_ms: after.as_millis(),
            signal,
            diagnosis: blocked.diagnose(),
        }),
    }
}

/// What a blocked capture means, which differs by mode: in an agent desktop it
/// has one documented cause.
enum Blocked {
    Host {
        output: String,
    },
    Agent {
        session: String,
        host_output: String,
        workspace: String,
        /// The console address while the desktop is shown: the invariant then
        /// holds on the user's own workspace, not on the headless output, and a
        /// diagnosis pointing at the headless one would name the wrong place.
        shown_console: Option<String>,
    },
}

impl Blocked {
    fn diagnose(&self) -> String {
        match self {
            Self::Host { output } => format!(
                "output {output} stopped answering screencopy requests; check `hyprctl monitors` \
                 and that the session window is still visible on it"
            ),
            Self::Agent {
                session,
                host_output,
                workspace,
                shown_console,
            } => {
                let site = shown_console.as_deref().map_or(
                    isolated::FrameSite::Headless {
                        output: host_output,
                        workspace,
                    },
                    |console| isolated::FrameSite::Shown { console },
                );
                isolated::frames_reason(session, &site, &isolated::frames_observation(&site))
            }
        }
    }
}

/// Window (or `--full` output) capture context for the active session.
struct Frame {
    /// `WAYLAND_DISPLAY` grim must use; `None` = the one this process inherited.
    display: Option<String>,
    output_name: String,
    geometry: Option<String>,
    blocked: Blocked,
}

fn ensure_capture_visible(expected_workspace: &str, monitor: &hypr::Monitor) -> Result<(), Error> {
    if !monitor.special_workspace.is_empty() {
        return Err(Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!(
                "special workspace {} is visible on output {} — captures would hide the session \
                 window",
                monitor.special_workspace, monitor.name
            ),
        });
    }
    if monitor.active_workspace.name != expected_workspace {
        return Err(Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!(
                "workspace {expected_workspace} is not active on output {} (active: {}) — \
                 captures would not show the window",
                monitor.name, monitor.active_workspace.name
            ),
        });
    }
    Ok(())
}

impl Frame {
    /// Routed by mode before the first compositor read: an agent desktop is
    /// captured through its own compositor, so asking the host would produce a
    /// plausible, wrong image.
    fn for_session(name: &str, full: bool) -> Result<Self, Error> {
        let session = session::load(name)?;
        match &session.state {
            ModeState::Shared(shared) => Self::host(shared, full),
            ModeState::Isolated(isolated) => Self::agent(name, isolated, full),
        }
    }

    fn host(shared: &session::Shared, full: bool) -> Result<Self, Error> {
        let (current, window) = session::shared_window(shared)?;
        let monitor = session::find_output(&current.output)?.ok_or_else(|| Error::Tool {
            command: "hyprctl monitors".to_owned(),
            message: format!("session output {} no longer exists", current.output),
        })?;
        // grim captures screen regions: if the parked workspace is not the
        // active one on the headless output, the capture would silently show
        // the wallpaper instead of the window.
        ensure_capture_visible(&current.workspace, &monitor)?;
        let geometry = if full {
            None
        } else {
            Some(crop_geometry(window.at, window.size, &monitor)?)
        };
        Ok(Self {
            display: None,
            blocked: Blocked::Host {
                output: monitor.name.clone(),
            },
            output_name: monitor.name,
            geometry,
        })
    }

    /// Grim on the nested compositor's display, framed on the session's window
    /// inside it; `--full` is the whole agent desktop.
    fn agent(name: &str, isolated: &session::Isolated, full: bool) -> Result<Self, Error> {
        let target = isolated::capture_target(name, isolated)?;
        let geometry = if full {
            None
        } else {
            Some(crop_geometry(
                target.window.at,
                target.window.size,
                &target.monitor,
            )?)
        };
        Ok(Self {
            display: Some(target.wayland_display),
            output_name: target.monitor.name,
            geometry,
            blocked: Blocked::Agent {
                session: name.to_owned(),
                host_output: isolated.output.clone(),
                workspace: isolated.workspace.clone(),
                shown_console: isolated.shown.then(|| target.console.clone()),
            },
        })
    }

    fn capture(&self, dest: &Path) -> Result<(), Error> {
        let dest_str = dest.to_string_lossy();
        let display = self.display.as_deref();
        self.geometry.as_ref().map_or_else(
            || {
                grim(
                    display,
                    &["-o", &self.output_name, &dest_str],
                    &self.blocked,
                )
            },
            |geometry| grim(display, &["-g", geometry, &dest_str], &self.blocked),
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

/// Captures and scratch files live under the session directory: two agent
/// desktops driven in parallel would otherwise overwrite each other's files.
/// `--out` still wins for `shot`.
fn output_dir(session_dir: &Path, out: Option<&Path>) -> PathBuf {
    out.map_or_else(|| session_dir.join("shots"), Path::to_path_buf)
}

fn scratch_paths(session_dir: &Path) -> [PathBuf; 2] {
    [
        session_dir.join("wait-a.png"),
        session_dir.join("wait-b.png"),
    ]
}

pub fn shot(
    session_name: &str,
    name: Option<&str>,
    full: bool,
    out_dir: Option<&Path>,
) -> Result<String, Error> {
    let frame = Frame::for_session(session_name, full)?;

    let dir = output_dir(&session::session_dir(session_name)?, out_dir);
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
    if value <= 0.0 {
        return Err(invalid());
    }
    // `try_from_secs_f64` is what rejects infinities and anything past
    // `Duration`'s range; its infallible sibling panics on both.
    Duration::try_from_secs_f64(value).map_err(|_| invalid())
}

/// `ready` means the window is capturable, so an agent desktop start proves it
/// with a real capture through the socket it just recorded rather than
/// inferring it from what the compositors answer. The image is thrown away —
/// what it proves is that grim reached that `WAYLAND_DISPLAY` and got a frame
/// out of it, which is what a socket attributed to the wrong instance fails at.
pub fn probe(session_name: &str) -> Result<(), Error> {
    let frame = Frame::for_session(session_name, false)?;
    let dir = session::session_dir(session_name)?;
    fs::create_dir_all(&dir).map_err(|source| Error::Io {
        context: format!("creating {}", dir.display()),
        source,
    })?;
    let path = dir.join("ready.png");
    let captured = frame.capture(&path).and_then(|()| read_png(&path));
    let _ = fs::remove_file(&path);
    captured.map(|_| ())
}

pub fn wait(session_name: &str, mode: &WaitMode, timeout: Duration) -> Result<String, Error> {
    // Resolved once here so a session that cannot be captured at all fails before
    // anything is written; the loop resolves it again on every iteration.
    let frame = Frame::for_session(session_name, false)?;
    let dir = session::session_dir(session_name)?;
    fs::create_dir_all(&dir).map_err(|source| Error::Io {
        context: format!("creating {}", dir.display()),
        source,
    })?;
    let poll_paths = scratch_paths(&dir);

    let started = Instant::now();
    let result = wait_loop(session_name, frame, mode, timeout, &poll_paths, started);
    for path in &poll_paths {
        // Best-effort scratch cleanup; the wait result is what matters.
        let _ = fs::remove_file(path);
    }
    result
}

/// Two captures only mean something about the same window when they were framed
/// the same way. A freshly mapped window emits late events and resizes, which is
/// exactly what `wait --stable` is used on right after a start: comparing through
/// a stale crop would call a frame stable once the wallpaper around the window
/// stopped moving.
#[derive(Default)]
struct Stability {
    framing: Option<String>,
    previous: Option<Image>,
}

impl Stability {
    fn observe(&mut self, framing: Option<String>, current: Image) -> bool {
        let reframed = self.framing != framing;
        self.framing = framing;
        if reframed {
            self.previous = Some(current);
            return false;
        }
        let stable = self
            .previous
            .as_ref()
            .is_some_and(|previous| previous.identical(&current));
        self.previous = Some(current);
        stable
    }
}

fn wait_loop(
    session_name: &str,
    first: Frame,
    mode: &WaitMode,
    timeout: Duration,
    poll_paths: &[PathBuf; 2],
    started: Instant,
) -> Result<String, Error> {
    let reference = match mode {
        WaitMode::Stable => None,
        WaitMode::ChangedFrom(path) => Some(read_png(path)?),
    };

    let mut frame = first;
    let mut stability = Stability::default();
    let mut captures: u32 = 0;
    loop {
        if captures > 0 {
            thread::sleep(WAIT_POLL_INTERVAL);
            // The window may have moved or resized since the last capture, so the
            // crop is read again rather than reused.
            frame = Frame::for_session(session_name, false)?;
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
        } else if stability.observe(frame.geometry.clone(), current) {
            return Ok(format!(
                "stable after {}ms ({captures} capture(s))",
                started.elapsed().as_millis()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Blocked, Image, Ran, Stability, crop_geometry, ensure_capture_visible, output_dir,
        parse_timeout, read_png, run_bounded, scratch_paths,
    };
    use crate::error::Error;
    use crate::hypr::Monitor;
    use crate::isolated;
    use crate::session::Escalation;
    use std::error::Error as StdError;
    use std::fs;
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    fn monitor(x: f64, y: f64, width: f64, height: f64) -> Result<Monitor, Box<dyn StdError>> {
        let json = format!(
            r#"{{"id": 1, "name": "headless-ci", "width": {width}, "height": {height},
                "x": {x}, "y": {y}, "scale": 1.0, "transform": 0, "focused": false,
                "activeWorkspace": {{"id": -1, "name": "proto"}},
                "specialWorkspace": {{"id": 0, "name": ""}}, "disabled": false}}"#
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
    fn timeout_out_of_duration_range_is_rejected_not_panicked() {
        // Finite, positive, and still past `Duration`'s range: the infallible
        // conversion panics on these instead of reporting a bad flag value.
        assert!(parse_timeout("2e19s").is_err());
        assert!(parse_timeout("1e400s").is_err());
        assert!(parse_timeout("nans").is_err());
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

    /// The real deadline is 5s; these tests need the same ladder, faster.
    const TEST_ESCALATION: Escalation = Escalation {
        polite: Duration::from_millis(60),
        term: Duration::from_millis(60),
        kill: Duration::from_millis(60),
        poll: Duration::from_millis(5),
    };

    fn bounded(script: &str) -> Result<Ran, Box<dyn StdError>> {
        let mut child = Command::new("sh")
            .args(["-c", script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(run_bounded(&mut child, TEST_ESCALATION, script)?)
    }

    #[test]
    fn a_capture_that_returns_is_left_alone() -> Result<(), Box<dyn StdError>> {
        let Ran::Exited { status, stderr } = bounded("exit 0")? else {
            return Err("a process that exited was reported as killed".into());
        };
        assert!(status.success());
        assert!(stderr.is_empty(), "{stderr}");

        let Ran::Exited { status, stderr } = bounded("echo 'no wl_output' >&2; exit 3")? else {
            return Err("a failing process was reported as killed".into());
        };
        assert!(!status.success());
        assert!(stderr.contains("no wl_output"), "{stderr}");
        Ok(())
    }

    #[test]
    fn a_blocked_capture_is_terminated_at_its_deadline() -> Result<(), Box<dyn StdError>> {
        let Ran::Killed { signal, after } = bounded("sleep 1")? else {
            return Err("a process past its deadline was reported as exited".into());
        };

        assert_eq!(signal, "TERM");
        assert!(
            after >= TEST_ESCALATION.polite && after < Duration::from_secs(1),
            "killed after {after:?}, expected just past the deadline"
        );
        Ok(())
    }

    #[test]
    fn a_capture_that_ignores_sigterm_is_killed() -> Result<(), Box<dyn StdError>> {
        let Ran::Killed { signal, after } = bounded("trap '' TERM; sleep 1")? else {
            return Err("a process ignoring SIGTERM was reported as exited".into());
        };

        assert_eq!(signal, "KILL");
        assert!(
            after >= TEST_ESCALATION.polite + TEST_ESCALATION.term,
            "killed after {after:?}, expected after the SIGTERM grace period"
        );
        Ok(())
    }

    #[test]
    fn captures_and_scratch_files_are_per_session() {
        let alpha = Path::new("/run/user/1000/hyprpilot/sessions/alpha");
        let beta = Path::new("/run/user/1000/hyprpilot/sessions/beta");

        assert_eq!(output_dir(alpha, None), alpha.join("shots"));
        // `--out` still wins for `shot`.
        let out = Path::new("/tmp/hyprpilot-shots");
        assert_eq!(output_dir(alpha, Some(out)), out);

        assert_eq!(
            scratch_paths(alpha),
            [alpha.join("wait-a.png"), alpha.join("wait-b.png")]
        );
        // Two agent desktops driven in parallel share no file.
        assert_ne!(scratch_paths(alpha), scratch_paths(beta));
        assert_ne!(output_dir(alpha, None), output_dir(beta, None));
    }

    #[test]
    fn stability_only_compares_captures_framed_the_same_way() {
        let image = |byte: u8| Image {
            width: 2,
            height: 1,
            data: vec![byte, byte, byte, 255, byte, byte, byte, 255],
        };
        let crop = |geometry: &str| Some(geometry.to_owned());
        let mut stability = Stability::default();

        // One capture is never stable, two identical ones through the same crop are.
        assert!(!stability.observe(crop("0,0 800x600"), image(1)));
        assert!(stability.observe(crop("0,0 800x600"), image(1)));

        // A window that moved or resized between two captures: the frames may well
        // be byte-identical (a window on a uniform background, shifted) while the
        // crop now covers something else, so the comparison starts over instead of
        // reporting a window that has not settled as stable.
        let mut settling = Stability::default();
        assert!(!settling.observe(crop("0,0 800x600"), image(1)));
        assert!(!settling.observe(crop("10,10 800x600"), image(1)));
        assert!(!settling.observe(crop("10,10 820x600"), image(1)));
        assert!(settling.observe(crop("10,10 820x600"), image(1)));

        // A frame that changed under an unchanged crop is not stable either.
        let mut animating = Stability::default();
        assert!(!animating.observe(crop("0,0 800x600"), image(1)));
        assert!(!animating.observe(crop("0,0 800x600"), image(2)));
        assert!(animating.observe(crop("0,0 800x600"), image(2)));
    }

    #[test]
    fn a_capture_timeout_names_the_kill_and_the_broken_invariant() {
        let error = Error::CaptureTimeout {
            command: "grim -g 0,0 1280x720 /run/user/1000/hyprpilot/sessions/alpha/shots/a.png"
                .to_owned(),
            after_ms: 7000,
            signal: "KILL",
            diagnosis: isolated::frozen_reason(
                "alpha",
                "hyprpilot-alpha",
                "agent-alpha",
                "workspace 3 is active on hyprpilot-alpha, not agent-alpha",
            ),
        }
        .to_string();

        assert!(error.contains("produced no frame within 7000ms"), "{error}");
        assert!(error.contains("SIGKILL"), "{error}");
        // The frame-callback cause and the documented host-side fallback.
        assert!(error.contains("frame callbacks"), "{error}");
        assert!(error.contains("grim -o hyprpilot-alpha"), "{error}");
    }

    #[test]
    fn a_blocked_shared_capture_points_at_its_own_output() {
        let diagnosis = Blocked::Host {
            output: "hyprpilot".to_owned(),
        }
        .diagnose();

        assert!(diagnosis.contains("output hyprpilot"), "{diagnosis}");
        assert!(diagnosis.contains("screencopy"), "{diagnosis}");
    }

    #[test]
    fn capture_refuses_visible_special_workspace() -> Result<(), Box<dyn StdError>> {
        let mut monitor = monitor(5120.0, 0.0, 1600.0, 1000.0)?;
        monitor.active_workspace.name = "proto".to_owned();
        monitor.special_workspace = "special:hyprpilot-parked".to_owned();

        let error = ensure_capture_visible("proto", &monitor)
            .err()
            .ok_or("visible special workspace unexpectedly accepted")?;
        assert!(error.to_string().contains("special workspace"));
        assert!(error.to_string().contains("captures would hide"));
        Ok(())
    }
}
