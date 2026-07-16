use std::env;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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

#[derive(Subcommand)]
enum Command {
    /// Manage the driving session (one at a time)
    #[command(subcommand)]
    Session(SessionCommand),
    /// Send key chords to the session window (no focus needed)
    Key {
        /// Chords like `a`, `Down`, `Ctrl+c`, `Ctrl+Shift+Escape`
        #[arg(required = true)]
        keys: Vec<String>,
        /// Delay between chords, in milliseconds
        #[arg(long, default_value_t = 50)]
        delay_ms: u64,
    },
    /// Type text character by character into the session window
    Type {
        text: String,
        /// Delay between characters, in milliseconds
        #[arg(long, default_value_t = 25)]
        delay_ms: u64,
    },
    /// Click in the session window (cursor and focus are restored)
    Click {
        /// X, relative to the window unless --absolute
        x: i32,
        /// Y, relative to the window unless --absolute
        y: i32,
        #[arg(long, value_enum, default_value_t = ButtonArg::Left)]
        button: ButtonArg,
        /// Treat X Y as global layout coordinates
        #[arg(long)]
        absolute: bool,
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
    /// Check the environment (hyprctl, grim, protocols, permissions)
    Doctor,
    /// End the session: close a spawned app (or return an attached window
    /// to its original workspace), remove the output
    Teardown {
        /// Kill the spawned process group instead of closing its window
        #[arg(long)]
        kill: bool,
        /// Close the window even if it was attached, not spawned
        #[arg(long)]
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
        Command::Key { keys, delay_ms } => crate::keys::send_keys(&keys, delay_ms),
        Command::Type { text, delay_ms } => keys::type_text(&text, delay_ms),
        Command::Click {
            x,
            y,
            button,
            absolute,
        } => pointer::click(x, y, button.into(), absolute),
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
        Command::Status => session::status(),
        Command::Doctor => doctor(),
        Command::Teardown { kill, close } => session::teardown(kill, close),
    }
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
