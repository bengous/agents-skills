//! The one place a durable change to the user's compositor is posed.
//!
//! `hyprctl` has five mutating calls this crate uses and two mutating
//! dispatchers. The calls are `pub(in crate::host)`, so outside `src/host/` they
//! do not exist and the wrappers below are the only way to reach them — the
//! invariant is the compiler's, not a convention. `hypr::dispatch` has to stay
//! visible to the crate (21 legitimate, read-only-in-effect sites), so for the
//! two dispatchers the same invariant is held by a source sweep instead
//! (`no_module_outside_host_dispatches_renameworkspace_or_moveworkspacetomonitor`).
//!
//! What each mutation costs to undo lives next door, in `ledger`.

pub mod hypr;
pub mod ledger;

use std::io::Write as _;

use crate::error::Error;

pub fn output_create_headless(output: &str) -> Result<(), Error> {
    hypr::output_create_headless(output)
}

pub fn output_remove(output: &str) -> Result<(), Error> {
    hypr::output_remove(output)
}

pub fn keyword_monitor(output: &str, width: u32, height: u32) -> Result<(), Error> {
    hypr::keyword_monitor(output, width, height)
}

pub fn keyword_monitor_at(
    output: &str,
    width: u32,
    height: u32,
    at: (i32, i32),
    scale: f64,
) -> Result<(), Error> {
    hypr::keyword_monitor_at(output, width, height, at, scale)
}

pub fn keyword_workspace(workspace: &str, monitor: &str) -> Result<(), Error> {
    hypr::keyword_workspace(workspace, monitor)
}

/// The rule text `keyword_workspace` posts, so a ledger entry records exactly
/// what went on the host.
pub fn workspace_rule(workspace: &str, monitor: &str) -> String {
    hypr::workspace_rule(workspace, monitor)
}

/// Whether the compositor already holds this rule. Hyprland stacks a repeated
/// rule instead of replacing it, the first one keeping precedence (#2268), and
/// nothing retracts either (#5691) — so a rule is posed at most once, and this
/// is the only defence there is.
///
/// A table that cannot be read or parsed answers `false`. A start must not die
/// because `hyprctl workspacerules` changed shape: the cost of being wrong here
/// is one leaked duplicate, which is what every start did unconditionally
/// before.
pub fn workspace_rule_is_posted(workspace: &str, monitor: &str) -> bool {
    rule_is_posted(hypr::workspace_rules(), workspace, monitor)
}

/// Split from the read above so the decision a broken table forces — post, do
/// not refuse — is assertable without a compositor.
fn rule_is_posted(
    rules: Result<Vec<hypr::WorkspaceRule>, Error>,
    workspace: &str,
    monitor: &str,
) -> bool {
    match rules {
        Ok(rules) => rules.iter().any(|rule| rule.binds(workspace, monitor)),
        Err(error) => {
            let _ = writeln!(
                std::io::stderr(),
                "hyprpilot: warning: could not read `hyprctl workspacerules` ({error}); posting \
                 the rule for {workspace} anyway, which leaks a duplicate until `hyprctl reload`"
            );
            false
        }
    }
}

/// Takes the workspace *id*, never its name: the name is what the rename is
/// about to change, and the id is what survives it.
pub fn rename_workspace(id: i64, to: &str) -> Result<(), Error> {
    hypr::dispatch(&["renameworkspace", &id.to_string(), to])
}

/// Takes a workspace *selector* (`session::workspace_selector`), because the
/// dispatcher does.
pub fn move_workspace_to_monitor(selector: &str, monitor: &str) -> Result<(), Error> {
    hypr::dispatch(&["moveworkspacetomonitor", selector, monitor])
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// `hyprctl workspacerules -j` as 0.56 answers it, captured live on
    /// 2026-07-27 with one rule posed.
    const WORKSPACE_RULES_JSON: &str = r#"[{
        "workspaceString": "agent-probe",
        "enabled": true,
        "monitor": "hyprpilot-probe",
        "default": true
    }]"#;

    /// Hyprland stacks a repeated rule instead of replacing it (#2268) and
    /// retracts none of them (#5691), so the equivalence check is the only thing
    /// standing between a session and a rule per start — thirty of them by the
    /// time the leak was found.
    #[test]
    fn an_equivalent_workspace_rule_is_posed_once() -> Result<(), Box<dyn StdError>> {
        let table: Vec<super::hypr::WorkspaceRule> = serde_json::from_str(WORKSPACE_RULES_JSON)?;
        let posted = |workspace: &str, monitor: &str| {
            super::rule_is_posted(Ok(table.clone()), workspace, monitor)
        };

        assert!(posted("agent-probe", "hyprpilot-probe"));
        assert!(
            !posted("agent-probe", "hyprpilot-other"),
            "the same workspace on another output is a different rule"
        );
        assert!(
            !posted("agent-other", "hyprpilot-probe"),
            "another workspace on the same output is a different rule"
        );
        assert!(!posted("special:hyprpilot-parked", "hyprpilot"));
        Ok(())
    }

    /// A start must not die because `hyprctl workspacerules` changed shape. The
    /// cost of answering "not posted" wrongly is one duplicate, which is what
    /// every start leaked unconditionally before the check existed.
    #[test]
    fn an_unreadable_rule_table_does_not_fail_a_start() {
        let unreadable = Err(crate::error::Error::Tool {
            command: "hyprctl workspacerules -j".to_owned(),
            message: "unknown field `workspaceString`".to_owned(),
        });
        assert!(
            !super::rule_is_posted(unreadable, "agent-alpha", "hyprpilot-alpha"),
            "a table this build cannot read has to make the caller post the rule, \
             not refuse the start: the cost is one duplicate, which is what every \
             start leaked before the check existed"
        );
    }

    /// The two mutating dispatchers, as a call site spells them. Backticks in a
    /// doc comment and the `hyprctl dispatch renameworkspace` of an error
    /// message do not match: only a quoted argument does.
    const MUTATING_DISPATCHERS: [&str; 2] = ["\"renameworkspace\"", "\"moveworkspacetomonitor\""];

    /// The parking rule used to be reposted by `activate_persisted_target`, so
    /// every `target` stacked another copy of it (#2268) with no way to retract
    /// any (#5691). It is a property of the output, so `start` poses it and
    /// nothing else does — a source fact, since the alternative is counting
    /// rules against a live compositor, which the gate does instead.
    #[test]
    fn the_parking_rule_is_posed_at_start_not_at_every_target() -> Result<(), Box<dyn StdError>> {
        let session =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/session.rs"))?;
        let activate = session
            .split_once("fn activate_persisted_target")
            .ok_or("activate_persisted_target is gone")?
            .1
            .split_once("\nfn ")
            .ok_or("activate_persisted_target has no end")?
            .0;
        assert!(
            !activate.contains("bind_workspace") && !activate.contains("keyword_workspace"),
            "a change of target must not repost the parking rule"
        );

        let start = session
            .split_once("\npub fn start(")
            .ok_or("session::start is gone")?
            .1;
        assert!(
            start.contains("bind_workspace(PARKING_WORKSPACE_NAME, OUTPUT_NAME)"),
            "the parking rule has to be posed once, where the output is created"
        );
        Ok(())
    }

    fn rust_files(dir: &Path, skip: &Path, into: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path == skip {
                continue;
            }
            if path.is_dir() {
                rust_files(&path, skip, into)?;
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                into.push(path);
            }
        }
        Ok(())
    }

    /// `pub(in crate::host)` keeps the five mutating `hyprctl` calls inside this
    /// module; the two dispatchers go through `hypr::dispatch`, which the crate
    /// needs for its twenty-one harmless uses. This is what stands in for the
    /// compiler there.
    #[test]
    fn no_module_outside_host_dispatches_renameworkspace_or_moveworkspacetomonitor()
    -> Result<(), Box<dyn StdError>> {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &src.join("host"), &mut files)?;
        assert!(
            files.len() > 5,
            "the sweep found {} files under {} — it is looking in the wrong place",
            files.len(),
            src.display()
        );

        for file in files {
            let text = fs::read_to_string(&file)?;
            for dispatcher in MUTATING_DISPATCHERS {
                assert!(
                    !text.contains(dispatcher),
                    "{} dispatches {dispatcher} itself; a durable host mutation is posed from \
                     src/host/ or not at all",
                    file.display()
                );
            }
        }
        Ok(())
    }
}
