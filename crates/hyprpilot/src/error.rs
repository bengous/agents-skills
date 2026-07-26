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
    /// A shared-mode-only code path reached with an isolated session. Every
    /// command routes by mode before its first compositor read, so this is a
    /// routing bug — typed, rather than a silent fall-through that would drive
    /// the user's own windows.
    ModeRouting {
        command: &'static str,
    },
    IsolatedUnsupported {
        command: &'static str,
        hint: &'static str,
    },
    /// The mirror of `IsolatedUnsupported`: a command only an agent desktop can
    /// answer, asked of a shared session.
    SharedUnsupported {
        command: &'static str,
        hint: &'static str,
    },
    /// The agent-desktop marker is set, so this process runs inside a nested
    /// Hyprland where the headless machinery cannot work.
    NestedRefused {
        session: String,
    },
    /// A process of the agent desktop survived its teardown, so the headless
    /// output must stay: removing it would drop the console on the user.
    AgentDesktopAlive {
        session: String,
        detail: String,
    },
    /// The state describes an agent desktop no command can act on: no nested
    /// compositor, or no window recorded yet.
    AgentDesktopUnready {
        session: String,
        reason: String,
    },
    /// The recorded nested compositor is gone (crash, or killed by hand): every
    /// isolated command refuses here instead of timing out on an instance that
    /// will never answer, or falling back to the user's desktop.
    AgentDesktopDead {
        session: String,
        signature: String,
        pid: u32,
    },
    /// Teardown removed everything it owns but one restoration step failed; the
    /// summary still says what was cleaned, so the session is known to be gone.
    TeardownIncomplete {
        summary: String,
        failures: Vec<RestoreFailure>,
    },
    /// A capture outlived its bounded deadline and was killed. In an agent
    /// desktop this has one documented cause, named by `diagnosis`.
    CaptureTimeout {
        command: String,
        after_ms: u128,
        signal: &'static str,
        diagnosis: String,
    },
    /// An output named after this session already exists: a leftover, never a
    /// resource to reuse.
    AgentOutputExists {
        output: String,
        session: String,
    },
    /// The user's desktop moved while an agent desktop was being built.
    HostDeviation {
        what: &'static str,
        expected: String,
        actual: String,
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
            Self::SharedSessionExists { name } => write_shared_session_exists(f, name),
            Self::CorruptSession { path, message } => write_corrupt_session(f, path, message),
            Self::UnsupportedSessionVersion { path, found } => {
                write_unsupported_version(f, path, *found)
            }
            Self::ModeRouting { command } => write_mode_routing(f, command),
            Self::IsolatedUnsupported { command, hint } => {
                write_unsupported(f, command, "isolated", hint)
            }
            Self::SharedUnsupported { command, hint } => {
                write_unsupported(f, command, "shared", hint)
            }
            Self::NestedRefused { session } => write_nested_refused(f, session),
            Self::AgentDesktopAlive { session, detail } => {
                write_agent_desktop_alive(f, session, detail)
            }
            Self::AgentDesktopUnready { session, reason } => {
                write!(f, "agent desktop `{session}` is not ready: {reason}")
            }
            Self::AgentDesktopDead {
                session,
                signature,
                pid,
            } => write_agent_desktop_dead(f, session, signature, *pid),
            Self::TeardownIncomplete { summary, failures } => {
                write!(f, "{summary}")?;
                write_restore_failures(f, "but the desktop was not fully restored:", failures)
            }
            Self::CaptureTimeout {
                command,
                after_ms,
                signal,
                diagnosis,
            } => write_capture_timeout(f, command, *after_ms, signal, diagnosis),
            Self::AgentOutputExists { output, session } => {
                write_agent_output_exists(f, output, session)
            }
            Self::HostDeviation {
                what,
                expected,
                actual,
            } => write_host_deviation(f, what, expected, actual),
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
            Self::Guarded { action, restore } => write_guarded(f, action.as_deref(), restore),
            Self::Timeout { what, after_ms } => {
                write!(f, "timed out waiting for {what} after {after_ms}ms")
            }
            Self::DoctorFailed { report, failures } => {
                write!(f, "{report}\n{failures} check(s) failed")
            }
        }
    }
}

fn write_mode_routing(f: &mut fmt::Formatter<'_>, command: &str) -> fmt::Result {
    write!(
        f,
        "internal error: `{command}` reached the shared-mode path with an agent desktop session; \
         no compositor state was touched — every command routes by mode before its first \
         compositor read, so this is a bug worth reporting"
    )
}

fn write_unsupported(
    f: &mut fmt::Formatter<'_>,
    command: &str,
    mode: &str,
    hint: &str,
) -> fmt::Result {
    write!(
        f,
        "`{command}` is not supported for {mode} sessions — {hint}"
    )
}

fn write_nested_refused(f: &mut fmt::Formatter<'_>, session: &str) -> fmt::Result {
    write!(
        f,
        "{} is set (agent desktop `{session}`): a headless output created inside a nested Hyprland \
         stays 0x0, so an agent desktop cannot be built from inside another one — run this from \
         the user's session",
        crate::isolated::AGENT_SESSION_ENV
    )
}

fn write_agent_output_exists(
    f: &mut fmt::Formatter<'_>,
    output: &str,
    session: &str,
) -> fmt::Result {
    write!(
        f,
        "output {output} already exists — it is left over from an earlier agent desktop, not a \
         resource to reuse; run `hyprpilot --session {session} teardown` (or `hyprctl output \
         remove {output}` if no session state is left)"
    )
}

fn write_shared_session_exists(f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
    write!(
        f,
        "shared session `{name}` is already active and the shared output `{}` is a singleton — run \
         `hyprpilot --session {name} teardown` first, or start an agent desktop with `--isolated`",
        crate::session::OUTPUT_NAME
    )
}

fn write_corrupt_session(
    f: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
    message: &str,
) -> fmt::Result {
    write!(
        f,
        "session file {} is corrupt ({message}); no output was removed. ",
        path.display()
    )?;
    write_session_recovery(f)
}

fn write_agent_desktop_alive(
    f: &mut fmt::Formatter<'_>,
    session: &str,
    detail: &str,
) -> fmt::Result {
    write!(
        f,
        "agent desktop `{session}` still has live processes ({detail}) — refusing to remove its \
         headless output, since its console window would land on the user's desktop; run \
         `hyprpilot --session {session} teardown` again"
    )
}

fn write_agent_desktop_dead(
    f: &mut fmt::Formatter<'_>,
    session: &str,
    signature: &str,
    pid: u32,
) -> fmt::Result {
    write!(
        f,
        "agent desktop `{session}` is dead: its nested compositor (instance {signature}, pid \
         {pid}) is gone, so nothing was sent anywhere — run `hyprpilot --session {session} \
         teardown` to remove its headless output and state, then start it again"
    )
}

fn write_capture_timeout(
    f: &mut fmt::Formatter<'_>,
    command: &str,
    after_ms: u128,
    signal: &str,
    diagnosis: &str,
) -> fmt::Result {
    write!(
        f,
        "`{command}` produced no frame within {after_ms}ms and was killed (SIG{signal}) — \
         {diagnosis}"
    )
}

fn write_host_deviation(
    f: &mut fmt::Formatter<'_>,
    what: &str,
    expected: &str,
    actual: &str,
) -> fmt::Result {
    write!(
        f,
        "the user's desktop changed while the agent desktop was starting ({what}: expected \
         {expected}, observed {actual}) — the start was rolled back"
    )
}

fn write_guarded(
    f: &mut fmt::Formatter<'_>,
    action: Option<&Error>,
    restore: &[RestoreFailure],
) -> fmt::Result {
    match action {
        Some(action) => write!(f, "action failed: {action}")?,
        None => write!(f, "action executed but desktop invariant violated")?,
    }
    write_restore_failures(f, "restoration failed:", restore)
}

fn write_restore_failures(
    f: &mut fmt::Formatter<'_>,
    header: &str,
    failures: &[RestoreFailure],
) -> fmt::Result {
    if failures.is_empty() {
        return Ok(());
    }
    write!(f, "\n{header}")?;
    for failure in failures {
        write!(
            f,
            "\n- {}: expected {}, actual {}",
            failure.what, failure.expected, failure.actual
        )?;
    }
    Ok(())
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
