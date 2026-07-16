use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Io {
        context: String,
        source: io::Error,
    },
    Json {
        context: String,
        source: serde_json::Error,
    },
    Png {
        context: String,
        message: String,
    },
    /// An external tool (hyprctl, grim, kill) failed.
    Tool {
        command: String,
        message: String,
    },
    Env(&'static str),
    NoSession,
    SessionExists(PathBuf),
    WindowNotFound(String),
    WindowGone(String),
    UnmappedChar(char),
    InvalidChord(String),
    Invalid {
        what: &'static str,
        value: String,
        hint: String,
    },
    Pointer(String),
    Timeout {
        what: String,
        after_ms: u128,
    },
    DoctorFailed {
        report: String,
        failures: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Json { context, source } => write!(f, "{context}: invalid JSON: {source}"),
            Self::Png { context, message } => write!(f, "{context}: {message}"),
            Self::Tool { command, message } => write!(f, "`{command}` failed: {message}"),
            Self::Env(name) => {
                write!(
                    f,
                    "environment variable {name} is not set — is this a Hyprland session?"
                )
            }
            Self::NoSession => {
                write!(f, "no active session — run `hyprpilot session start` first")
            }
            Self::SessionExists(path) => write!(
                f,
                "a session is already active ({}) — run `hyprpilot teardown` first",
                path.display()
            ),
            Self::WindowNotFound(criteria) => write!(f, "no window matches {criteria}"),
            Self::WindowGone(address) => write!(
                f,
                "session window {address} no longer exists — run `hyprpilot teardown`"
            ),
            Self::UnmappedChar(c) => write!(
                f,
                "character {c:?} has no keysym mapping — send it as a raw keysym with `hyprpilot key <keysym>`"
            ),
            Self::InvalidChord(chord) => write!(
                f,
                "invalid key chord `{chord}` — expected e.g. `a`, `Down`, `Ctrl+c`, `Ctrl+Shift+Escape`"
            ),
            Self::Invalid { what, value, hint } => {
                write!(f, "invalid {what} `{value}` — {hint}")
            }
            Self::Pointer(message) => write!(f, "virtual pointer: {message}"),
            Self::Timeout { what, after_ms } => {
                write!(f, "timed out waiting for {what} after {after_ms}ms")
            }
            Self::DoctorFailed { report, failures } => {
                write!(f, "{report}\n{failures} check(s) failed")
            }
        }
    }
}

impl std::error::Error for Error {}
