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
    SessionExists {
        name: String,
        path: PathBuf,
    },
    /// The shared output is a singleton, so a second shared session is refused
    /// whatever its name.
    SharedSessionExists {
        name: String,
    },
    CorruptSession {
        path: PathBuf,
        message: String,
    },
    /// `found` is `None` for the unversioned pre-v2 format, or when the file
    /// cannot be parsed far enough to tell.
    UnsupportedSessionVersion {
        path: PathBuf,
        found: Option<u32>,
    },
    /// Isolated mode is routed away from the shared code path until the slice
    /// that implements it lands.
    IsolatedPending {
        command: &'static str,
        slice: &'static str,
    },
    IsolatedUnsupported {
        command: &'static str,
        hint: &'static str,
    },
    SweepRefused {
        output: String,
        reason: String,
    },
    WindowAmbiguous {
        criteria: String,
        candidates: serde_json::Value,
    },
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
            Self::SessionExists { name, path } => write!(
                f,
                "session `{name}` is already active ({}) — run \
                 `hyprpilot --session {name} teardown` first",
                path.display()
            ),
            Self::SharedSessionExists { name } => write!(
                f,
                "shared session `{name}` is already active and the shared output \
                 `{}` is a singleton — run `hyprpilot --session {name} teardown` first, or start \
                 an agent desktop with `--isolated`",
                crate::session::OUTPUT_NAME
            ),
            Self::CorruptSession { path, message } => {
                write!(
                    f,
                    "session file {} is corrupt ({message}); no output was removed. ",
                    path.display()
                )?;
                write_session_recovery(f)
            }
            Self::UnsupportedSessionVersion { path, found } => {
                write_unsupported_version(f, path, *found)
            }
            Self::IsolatedPending { command, slice } => write!(
                f,
                "`{command}` is not implemented for isolated sessions yet (slice {slice}); no \
                 compositor state was touched"
            ),
            Self::IsolatedUnsupported { command, hint } => write!(
                f,
                "`{command}` is not supported for isolated sessions — {hint}"
            ),
            Self::SweepRefused { output, reason } => write!(
                f,
                "refusing to remove orphan output {output}: {reason}; no output was removed"
            ),
            Self::WindowAmbiguous {
                criteria,
                candidates,
            } => write!(
                f,
                "multiple windows match {criteria}; refine the match criteria\n{candidates}"
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

fn write_unsupported_version(
    f: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
    found: Option<u32>,
) -> fmt::Result {
    let expected = crate::session::SCHEMA_VERSION;
    let path = path.display();
    match found {
        Some(version) => write!(
            f,
            "session state {path} has schema version {version}, this build expects {expected}"
        )?,
        None => write!(
            f,
            "session state {path} is unversioned (pre-v2 format), this build expects schema \
             version {expected}"
        )?,
    }
    write!(
        f,
        " — no output was removed. `hyprpilot teardown` still clears a pre-v3 session left at \
         $XDG_RUNTIME_DIR/hyprpilot/session.json; a state file anywhere else has to be removed by \
         hand, once no window depends on it. "
    )?;
    write_session_recovery(f)
}

fn write_session_recovery(f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        f,
        "recover with `hyprpilot windows`, `hyprctl dispatch movetoworkspacesilent ...`, \
         `hyprctl dispatch closewindow ...`, then `hyprctl output remove hyprpilot`"
    )
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
