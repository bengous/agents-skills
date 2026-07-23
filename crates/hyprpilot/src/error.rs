use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub struct RestoreFailure {
    pub what: &'static str,
    pub expected: String,
    pub actual: String,
}

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
    UnsupportedSessionVersion(u32),
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
    Guarded {
        action: Option<Box<Self>>,
        restore: Vec<RestoreFailure>,
    },
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
            Self::UnsupportedSessionVersion(version) => write!(
                f,
                "session schema version {version} is unsupported (expected 2) — use a compatible \
                 hyprpilot version to run teardown"
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
            Self::Guarded { action, restore } => {
                if let Some(action) = action {
                    write!(f, "action failed: {action}")?;
                } else {
                    write!(f, "action executed but desktop invariant violated")?;
                }
                if !restore.is_empty() {
                    write!(f, "\nrestoration failed:")?;
                    for failure in restore {
                        write!(
                            f,
                            "\n- {}: expected {}, actual {}",
                            failure.what, failure.expected, failure.actual
                        )?;
                    }
                }
                Ok(())
            }
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

#[cfg(test)]
mod tests {
    use super::{Error, RestoreFailure};

    #[test]
    fn guarded_display_reports_action_failure_alone() {
        let error = Error::Guarded {
            action: Some(Box::new(Error::Pointer("click failed".to_owned()))),
            restore: Vec::new(),
        };

        assert_eq!(
            error.to_string(),
            "action failed: virtual pointer: click failed"
        );
    }

    #[test]
    fn guarded_display_reports_restoration_failure_alone() {
        let error = Error::Guarded {
            action: None,
            restore: vec![RestoreFailure {
                what: "cursor",
                expected: "(10, 20)".to_owned(),
                actual: "(12, 20)".to_owned(),
            }],
        };

        assert_eq!(
            error.to_string(),
            "action executed but desktop invariant violated\n\
             restoration failed:\n\
             - cursor: expected (10, 20), actual (12, 20)"
        );
    }

    #[test]
    fn guarded_display_reports_action_and_restoration_failures_together() {
        let error = Error::Guarded {
            action: Some(Box::new(Error::Pointer("scroll failed".to_owned()))),
            restore: vec![
                RestoreFailure {
                    what: "focus",
                    expected: "address:0xabc".to_owned(),
                    actual: "address:0xdef".to_owned(),
                },
                RestoreFailure {
                    what: "cursor",
                    expected: "(10, 20)".to_owned(),
                    actual: "(14, 25)".to_owned(),
                },
            ],
        };

        assert_eq!(
            error.to_string(),
            "action failed: virtual pointer: scroll failed\n\
             restoration failed:\n\
             - focus: expected address:0xabc, actual address:0xdef\n\
             - cursor: expected (10, 20), actual (14, 25)"
        );
    }

    #[test]
    fn guarded_display_reports_unrestorable_prior_no_focus_state() {
        let error = Error::Guarded {
            action: None,
            restore: vec![RestoreFailure {
                what: "focus",
                expected: "no focused window".to_owned(),
                actual: "address:0xabc; cannot restore prior no-focus state".to_owned(),
            }],
        };

        assert_eq!(
            error.to_string(),
            "action executed but desktop invariant violated\n\
             restoration failed:\n\
             - focus: expected no focused window, actual address:0xabc; \
             cannot restore prior no-focus state"
        );
    }
}
